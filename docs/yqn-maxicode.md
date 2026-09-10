# MaxiCode 接入切片

状态：IMPLEMENTED（引擎本地资格通过）；未发行、未接入 Components、未部署 Dev。

选择给自有 zpl-forge 补 MaxiCode：复用 Labelize v1.4.1 编码与参考向量，新增 ^BD 解析和中间指令，PDF 输出六边形及同心环矢量路径，PNG 复用相同几何。与给栅格补中文相比，不触动字体测宽、基线与换行，工作量没有明显更高。

规范：https://docs.zebra.com/us/en/printers/software/zpl-pg/c-zpl-zpl-commands/r-zpl-bd.html

首片模式 2/3/4，默认 2，^BY 不影响大小；暂不支持模式 5/6、Structured Append、反色，必须明确失败。保留原栅格版本。验收：参考编码向量、三模式中文同页 PDF/PNG 独立解码、^FH 控制字符、DPI 物理尺寸、错误输入、既有中文 Code128 回归、WASM 运行。

源码与许可证归属见 THIRD_PARTY_LICENSES.md。不得把本地测试写成 Dev 已发布。

## 验收结果（2026-09-11）

- 19 项库单测 + 2 项接口测试 + 2 项 doctest 通过；外置字体资格用例独立运行通过，不在普通 CI 假装加载了字体。
- 本地 workerd：模式 2/3/4 × 203/300/600 DPI × shaped PDF/PNG 共 18 次执行；每次实例创建/free。600 DPI 画布缩至 2000×2000 dots 以遵守现有 4,194,304 dots² 上限，未提高生产上限。
- PDF 原文包含中文；PDF 没有 Image XObject，MaxiCode 为原生路径。PNG 与 PDF 转图共 18 份用 ZXing-C++ 3.1.1 解码，原始 bytes 与模式 2/3 的邮编/国家/服务类别、RS/GS/EOT 及模式 4 文本完全一致。
- 解码器使用已知坐标截取独立条码区域；整页自动定位未通过；同类 rxing MaxiCodeReader 源码明确仅支持 pure barcode 提取。本次仍不把区域解码等同实机扫描、承运商认证或真实箱标全命令覆盖。
- 三份 203 DPI PDF 预览的 Code128 独立解码为 123456789012；中文预览已目视检查。
- 默认模式与 ^BY 不改变符号的测试通过；模式 5/6、多符号和非法参数明确拒绝。反色尚不支持并明确失败，不沿用旧栅格实现的声明。
- 可复用源约 640 行编码/表格，其余为命令接入、绘图、测试和归属说明；无新增 Rust 运行依赖。栅格补中文试探已撤回，没有同时推进两套改造。

## 复跑

```sh
cargo test --locked --features shaped-pdf
MAXICODE_CJK_FONT=/path/to/verified-font.ttf MAXICODE_OUTPUT_DIR=/tmp/maxicode-native cargo test --locked --features shaped-pdf --test maxicode -- --ignored
WASM_BINDGEN=/path/to/wasm-bindgen bash wasm/build.sh
MAXICODE_COMPONENTS_ROOT=/path/to/vendor-adapter-components MAXICODE_CJK_FONT=/path/to/verified-font.ttf MAXICODE_OUTPUT_DIR=/tmp/maxicode-workerd node wasm/maxicode-qualification.mjs
python3 wasm/verify-maxicode.py /tmp/maxicode-workerd
```

最后一步的隔离 Python 环境需 zxing-cpp 3.1.1、Pillow、PyMuPDF；Node runner 复用 Components 的已装 Miniflare，不新增引擎服务依赖。字体固定 SHA 在 runner 校验，不将字体字节提交。证据见同目录 maxicode-workerd-results.json 与 maxicode-decode-results.json。linear memory 是返回时数值，不代表进程峰值。

下一发布阶段需固定新的不可变包版本，更新 Components 冻结 descriptor/Profile 并完成 Dev Job→Artifact 在线验收。现有 VectorPdf@1/@2 的线上能力声明不变，不能因为此分支通过就删除 Labelize/EPL。
