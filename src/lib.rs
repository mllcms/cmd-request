use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process::exit,
    time::Duration,
};

use clap::Parser;
use colored::Colorize;
use reqwest::Method;
use reqwest::{
    header::{HeaderName, HeaderValue, CONTENT_TYPE},
    Body, Client,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    url: String,

    /// Query argument Example: -q name=zs -q age=18
    #[arg(short, long)]
    query: Vec<String>,

    /// Header argument Example: -H Content-Type=application/json
    #[arg(short = 'H', long)]
    header: Vec<String>,

    /// Body argument need onself set header
    #[arg(short, long)]
    body: Vec<String>,

    /// Body json argument and auto set -H Content-Type=application/json
    #[arg(short, long)]
    json: Vec<String>,

    /// Read json file and auto set -H Content-Type=application/json
    #[arg(short = 'J', long)]
    json_file: Option<PathBuf>,

    /// Body form argument and auto set -H Content-Type=application/x-www-form-urlencoded
    #[arg(short, long)]
    form: Vec<String>,

    /// Output response body to file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Request timeout Unit: s
    #[arg(short, long, default_value_t = 20)]
    timeout: u64,

    /// Show request and response Default: requset h(0) b(0) response h(0) b(1)
    #[arg(short, long, default_value_t = format!("0001"))]
    show: String,
}

impl Args {
    #[allow(dead_code)]
    pub async fn run(self, method: Method) -> anyhow::Result<()> {
        let client = Client::new();
        let ea = echo_model(self.show);

        let mut request = client
            .request(method.clone(), self.url)
            .timeout(Duration::from_secs(self.timeout));

        // set query
        let query: Vec<(_, _)> = self
            .query
            .iter()
            .map(|q| match q.split_once("=") {
                Some(res) => res,
                None => exit_fmt(format!("Parsing query [{q}] failed")),
            })
            .collect();
        request = request.query(&query);

        // set body 多种 body 可能会冲突所以只匹配一种
        if !self.body.is_empty() {
            request = request.body(self.body.join(""))
        } else if !self.json.is_empty() {
            let json: HashMap<_, _> = self
                .json
                .iter()
                .map(|j| match j.split_once("=") {
                    Some(res) => res,
                    None => exit_fmt(format!("Parsing json [{j}] failed")),
                })
                .collect();
            request = request.json(&json);
        } else if !self.form.is_empty() {
            let form: HashMap<_, _> = self
                .form
                .iter()
                .map(|f| match f.split_once("=") {
                    Some(res) => res,
                    None => exit_fmt(format!("Parsing form [{f}] failed")),
                })
                .collect();
            request = request.form(&form);
        } else if let Some(path) = self.json_file {
            if !is_ext(&path, "json") {
                exit_fmt("File format error must be json")
            }
            let file = fs::read_to_string(path).map_err(exit_fmt)?;
            request = request.json(&file)
        }

        // set header
        let mut req = request.build()?;
        let headers = req.headers_mut();
        for h in self.header {
            let (key, value) = match h.split_once("=") {
                Some(res) => res,
                None => exit_fmt(format!("Parsing header [{h}] failed")),
            };
            headers.insert(
                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }

        // 打印 requesd header
        if ea[0] == '1' {
            println!("{} {} {:?}", method, req.url(), req.version());
            println!("{}", "Request-Header:".truecolor(250, 208, 0));
            for (k, v) in req.headers() {
                println!(
                    "  {}: {}",
                    format!("{:?}", k).truecolor(224, 108, 117),
                    format!("{:?}", v).truecolor(152, 195, 121)
                )
            }
        }
        // 打印 requesd body
        if ea[1] == '1' {
            println!("{}", "Request-Body:".truecolor(250, 208, 0));
            if let Ok(body) = get_body(req.body()) {
                println!("  {}", body)
            }
            println!();
        }

        // 发起请求
        let res = client.execute(req).await?;
        let msg = match res.status().is_success() {
            true => format!("{} {:?} {}", method, res.version(), res.status()).green(),
            false => format!("{} {:?} {}", method, res.version(), res.status()).red(),
        };
        println!("{msg}");

        // 打印 response headers
        if ea[2] == '1' {
            println!("{}", "Response-Header:".truecolor(250, 208, 0));
            for (k, v) in res.headers() {
                println!(
                    "  {}: {}",
                    format!("{:?}", k).truecolor(224, 108, 117),
                    format!("{:?}", v).truecolor(152, 195, 121)
                )
            }
            println!()
        }

        // 获取响应类型和 body 数据
        let content_type = res.headers().get(CONTENT_TYPE).map(|h| h.clone());
        let body = res.bytes().await.unwrap_or(Default::default());
        // 打印 response body
        if ea[3] == '1' {
            println!("{}", "Response-Body:".truecolor(250, 208, 0));
            match content_type {
                Some(h) if h.as_bytes().include("json".as_bytes()) => {
                    format_json(&String::from_utf8_lossy(&body.to_vec()));
                }
                _ => {
                    println!("{:?}", String::from_utf8_lossy(&body.to_vec()))
                }
            }
        }
        if let Some(out) = self.output {
            let mut file = File::options().create(true).write(true).open(out)?;
            file.write(&body)?;
        }

        Ok(())
    }
}

// 输出模式
fn echo_model(s: String) -> [char; 4] {
    let mut res = ['0'; 4];
    for (i, c) in s.chars().enumerate() {
        if i >= res.len() {
            break;
        }
        match c {
            '0' | '1' => res[i] = c,
            _ => exit_fmt("Show format error Example: 0011 1111 0101"),
        }
    }
    res
}

fn get_body(body: Option<&Body>) -> anyhow::Result<Cow<'_, str>> {
    let body = body.ok_or(anyhow::Error::msg("Body is not exist"))?;
    let result = String::from_utf8_lossy(
        body.as_bytes()
            .ok_or(anyhow::Error::msg("Parsing body failed"))?,
    );
    Ok(result)
}

// 格式化 json
fn format_json(s: &str) {
    let mut buf = String::new(); // 缓冲区
    let mut sign = Vec::new(); // 符号栈主要是 '{' 和 '[' 用来控制缩进
    let mut prev = ' '; // 上一个字符
    let mut is_string = false; // 是否为 String

    for c in s.chars() {
        match c {
            '"' if !is_string => is_string = true,
            '"' if prev != '\\' => is_string = false, // 不匹配 \" 转义
            _ if is_string => buf.push(c),            // 匹配字符串类型
            ' ' | '\n' | '\r' => continue,            // 过滤空格和换行符
            ':' => echo(&mut buf, DateType::Key(sign.len())), // 输出 kye
            '{' => {
                // 符号 '{' 进栈并打印换行
                match sign.last() {
                    // 数组没有 key 属性所以要单独缩进
                    Some('[') => println!("{}{}", indent(sign.len()), "{"),
                    _ => println!("{}", "{"),
                }
                sign.push(c)
            }
            '[' => {
                // 符号 '{' 进栈并打印换行
                match sign.last() {
                    // 数组没有 key 属性所以要单独缩进
                    Some('[') => println!("{}{}", indent(sign.len()), "[",),
                    _ => println!("["),
                }
                sign.push(c)
            }
            // 边界符开始回栈
            ',' | '}' | ']' => {
                match (prev, sign.last()) {
                    ('"', Some('{')) => echo(&mut buf, DateType::String(0)), // value 不用空格
                    (_, Some('{')) => echo(&mut buf, DateType::KeyWord(0)),  // value 不用空格
                    ('"', _) => echo(&mut buf, DateType::String(sign.len())), // 字符串类型
                    _ if buf.is_empty() => {} // 缓冲区为空还不是字符串 基本 ',' 前面是 '}' 或 ']' 不做处理
                    _ => echo(&mut buf, DateType::KeyWord(sign.len())), // 关键字类型
                }
                if c == ',' {
                    println!(",") // 逗号不会栈直接打印并换行就行
                } else {
                    print!("\n{}{}", indent(sign.len() - 1), c);
                    sign.pop(); // 回栈减少下次缩进
                }
            }
            _ => buf.push(c),
        }
        prev = c;
    }
}

// 数据类型 主要是用来区分染色的
enum DateType {
    Key(usize),     // 对象的 key
    KeyWord(usize), // 关键字
    String(usize),  // String
}

// 格式化输出 + 染色，可以换成返回字符串收集
fn echo(buf: &mut String, t: DateType) {
    match t {
        DateType::Key(n) => print!(
            "{}{}",
            indent(n),
            &format!("{:#?}: ", buf).truecolor(224, 108, 117)
        ),
        DateType::KeyWord(n) => print!("{}{}", indent(n), &buf.truecolor(209, 154, 102)),
        DateType::String(n) => print!(
            "{}{}",
            indent(n),
            &format!("{:#?}", buf).truecolor(152, 195, 121)
        ),
    }
    buf.clear() // 清空缓冲区
}

// 缩进两个两个空格 可以切换
fn indent(n: usize) -> String {
    "  ".repeat(n)
}

/// 发散函数 退出主线程并提示错误信息
pub fn exit_fmt<T: Display>(s: T) -> ! {
    eprintln!("{s}");
    exit(0)
}

/// 判断扩展名
pub fn is_ext(path: &PathBuf, ext: &str) -> bool {
    if let Some(extension) = path.extension() {
        if let Some(ext_str) = extension.to_str() {
            return ext_str.to_lowercase().as_str() == ext;
        }
    }
    false
}

// 给 &[u8] 加是否包含另一个 &[u8]的方法
pub trait Include {
    fn include(&self, s: Self) -> bool;
}

impl Include for &[u8] {
    fn include(&self, s: &[u8]) -> bool {
        let mut index = 0;
        for b in self.iter() {
            if b == &s[index] {
                index += 1
            } else {
                index = 0
            }
            if index == s.len() {
                return true;
            }
        }
        false
    }
}
