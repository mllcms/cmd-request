## Rust 终端请求工具

### 打包
```bash
# 可执行文件在 target/debug
cargo build
# 可执行文件在 target/release
cargo build --release
```
### 使用
```bash 
req.exe get https://www.baidu.com
# 获取请求体并输出到 data.json 文件
req.exe get https://api.vvhan.com/api/covid -q city=北京 -o ./data.json

# 其他示例【没怎么测过】
# 多种请求 body 只会生效一种 -j -q 会自动设置对应 header 类型 -b 要自己设置 header
req.exe get xxx -q key=value -q key=value
req.exe post xxx -j key=value -j key=value
req.exe post xxx -f key=value -f key=value
req.exe post xxx -b xxxxxxxx -b xxxxxxx -H key=value
# 读取 josn 文件当请求体 body
req.exe post xxx -J [json文件路径]
# -s --show [请求头 请求体 响应头 响应体] 对应位置给 1 就会显示 默认 0001 只显示响应体 1111 就全部显示
req.exe get https://api.vvhan.com/api/covid -q city=北京 -s 0011
```
