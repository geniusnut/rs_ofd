# rs_ofd
An invoice ofd to image converter. [Rust-Lang]

```  
OFD发票转为PNG 参考GBT_33190-2016_电子文件存储与交换格式版式文档.pdf。
渲染后端使用skia 或者是 raqote.  
在Cargo.toml 中配置 features即可。

关于字体，Ubuntu需要将Windows下的KaiTi.ttf  simsunb.ttf  simsun.ttc拷贝安装。或者
```
```bash
cp  simsun.ttc ~/.local/share/fonts/
sudo mkfontscale
sudo mkfontdir
sudo fc-cache -fsv
```

## Usage

```bash
./ofd_demo 1.ofd 2.ofd ...
```

