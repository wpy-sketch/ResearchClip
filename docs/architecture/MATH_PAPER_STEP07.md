# Step07 数学论文Markdown契约

2026-10-24，基点9ac5702；先于业务实现写入。复查AGENTS、架构、保真投影、MathFormula/MarkdownPreview和原读写/撤销。仅Step07，SQLite12，无迁移、新权限、第二正文或全局资源ID。文内标签是文档局部锚点，不替代资源ID。

## 语法版本1

公式沿用$和$$及已安装KaTeX0.19.0。官网当前显示0.19.1，本轮以0.19.0实际测试为准。[支持列表](https://katex.org/docs/supported.html)、[选项](https://katex.org/docs/options.html)。继续trust=false、strict=error、独立宏、单式8192字符/全文256式或128KiB预算，失败显示原LaTeX。

定理容器必须从行首开始，以独占一行的三个冒号结束：

```md
<!-- rc-label: thm:limit -->
:::theorem 极限唯一性
设极限存在，则……

$$
\lim_{n\to\infty} a_n=L
$$
:::
```

八种名称definition/assumption/lemma/theorem/corollary/proof/example/counterexample，对应定义/假设/引理/定理/推论/证明/例子/反例。标题为同一行文本。不支持嵌套；未知、嵌套、未闭合容器保留原文并诊断。代码、公式、HTML中的容器示例不作为环境。正文支持原有Markdown、数学、表格和图片，不执行HTML。

标签为独占一行注释，其后只能有空白，关联下一个块公式/定理/独立图段/GFM表格。ASCII字母开始，其余为字母、数字、下划线、点、冒号、连字符，最多64字符，区分大小写。

```md
<!-- rc-label: eq:energy -->
$$
E=mc^2
$$

见[公式](#rc-ref:eq:energy)及[定理](#rc-ref:thm:limit)。
```

引用使用标准Markdown链接。显示原链接文字和当前编号，只在所属预览定位，不跨会话或授予文件权限。重复标签不猜测，缺失/孤立标签报错。普通编辑器仍保留完整容器、公式、注释和可读链接文本。

## 编号配置

```md
<!-- rc-paper {"version":1,"scope":"document","headingDepth":1,"theoremCounter":"shared","proofNumbered":false,"equationStart":1,"theoremStart":1,"figureStart":1,"tableStart":1} -->
```

最多一条配置；未知字段/未来版本/非法值拒绝套用并诊断。scope为document或section；section按headingDepth(1或2)的同级标题顺序划节，节前为0节，每节重置，显示节序号.局部序号。theoremCounter为shared或byType；proofNumbered默认false。四类起始值1–9999。所有块公式、环境、独立图段、表格按源码顺序计算，不必先有标签。行内图不计，同一独立图段可含多张子图。显式LaTeX tag保留，不叠加自动编号、不占自动计数；notag/nonumber不自动编号。复杂tag引用显示其源码文字，不重新执行TeX。

配置、标签、定理或rc-ref引用出现时才启用论文扩展，旧文档/指南不自动改变。编号只属于视图，插入/删除/移动后重算，不把数字写回。各可视片段与旧caption共享全文分析和原始偏移，不能逐块从1计数。支持的定理作为整个原片段移动，未知容器保留原有保护。

## 实现与验收

复用fromMarkdown+math；直接声明已安装同版本micromark-extension-gfm3.0.0与mdast-util-gfm3.1.0（MIT，原GFM许可已覆盖），不升级传递版本。官方API：[micromark GFM](https://github.com/micromark/micromark-extension-gfm)、[mdast GFM](https://github.com/syntax-tree/mdast-util-gfm)。只切分渲染片段，不序列化源文件。模板、公式和配置确认均走原editSource/editBlock，核对原会话和快照。

测试复杂矩阵/分式/求和/积分/aligned、错误和不可信命令、重复/缺失标签、代码排除、环境边界、重排/重新编号、CRLF和未知语法往返、多会话、保存/关闭及Windows重启。依赖声明做干净安装。物理IME/DPI及锁屏OLE不冒充通过。无PDF导出和独立备份恢复；v4.1.6发布完成后停止。
