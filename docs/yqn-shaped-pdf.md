# YQN 中文排版 PDF 切片

状态：ACCEPTED；实现及发布验收进行中。自有 fork 维护引擎代码，Components 维护 Profile/Job/Artifact。

新增 opt-in `renderShapedPdf`，原 PDF/PNG 字符布局不变。首片只对字体 0 使用受控静态 TrueType、Rustybuzz 0.20.1、Latin/CJK 左至右文本、字形定位与 cluster 原文映射。Unicode 分行、测宽与旋转使用同一布局；缺字、不受支持 UVS、RTL/非支持 script 和控制文本失败。PDF 子集保留 ToUnicode，复杂 cluster 用 ActualText。不得把不支持的变体选择符静默丢弃。

文件所有权：本片串行修改 engine/font、engine/shaping、engine/engine、forge/pdf_unicode、forge/pdf_native 和 wasm。保留已有解析器、条码与 raster backend。先通过 Rust/真实 workerd 的文字提取、旋转、换行、错误及旧路径回归，再发布 prerelease；Components 的 Dev 是独立验收。

后置：自动多字体 fallback、BIDI、全 Unicode、native shaped PNG、变量/CFF 字体。PDF 转图走既有后处理能力，不保证 raster 像素一致。不引入字体服务或系统字体依赖。

本地验收：8 项 Rust 单测与 2 项文档测试通过；真实 workerd 中文/组合音标/连字提取、四方向旋转、窄框分行、不支持 UVS/缺字/script 拒绝通过；4 份 Code128 独立解码通过。字体字节使用 Arc 共享避免 FontManager clone 再复制；样例返回时 linear memory 48955392 bytes（非峰值）。native Cargo.lock 同步到此前 WASM 发布锁基线，新增 shaping 依赖；没有把上游陈旧 native 锁当作当前发布基线。
