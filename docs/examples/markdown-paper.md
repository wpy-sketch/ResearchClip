# 数学论文练习：可读源码与稳定引用

<!-- rc-paper {"version":1,"scope":"document","headingDepth":2,"theoremCounter":"shared","proofNumbered":false,"equationStart":1,"theoremStart":1,"figureStart":1,"tableStart":1} -->

## 线性代数与多行推导

行内公式 $\lVert Ax\rVert_2\le\lVert A\rVert_2\lVert x\rVert_2$ 保持普通LaTeX。

<!-- rc-label: eq:matrix -->

$$
\begin{bmatrix}
\frac{1}{2} & \sum_{i=1}^{n}i \\
\int_0^1 x^2\,dx & \begin{pmatrix}a&b\\c&d\end{pmatrix}
\end{bmatrix}
$$

<!-- rc-label: eq:derivation -->

$$
\begin{aligned}
f(x)&=\sum_{n=0}^{\infty}\frac{x^n}{n!}\\
f'(x)&=\sum_{n=1}^{\infty}\frac{x^{n-1}}{(n-1)!}=f(x)
\end{aligned}
$$

见[矩阵](#rc-ref:eq:matrix)与[推导](#rc-ref:eq:derivation)。移动对象后标签不变，显示序号重算。

## 八种环境

<!-- rc-label: def:space -->

:::definition 线性空间
设 $V$ 是实数域上的线性空间。
:::

:::assumption 完备性
假设所用范数空间完备。
:::

:::lemma 三角不等式
对任意 $x,y\in V$，有 $\lVert x+y\rVert\le\lVert x\rVert+\lVert y\rVert$。
:::

<!-- rc-label: thm:unique -->

:::theorem 极限唯一性
收敛序列的极限唯一。由[定义](#rc-ref:def:space)在选定空间内讨论。
:::

:::corollary 常数列
常数序列收敛到其常数值。
:::

:::proof 唯一性证明
若 $a_n\to a$ 且 $a_n\to b$，则三角不等式给出 $\lVert a-b\rVert=0$，所以 $a=b$。
:::

:::example 几何级数
当 $|r|<1$ 时，$\sum_{n=0}^{\infty}r^n=(1-r)^{-1}$。
:::

:::counterexample 有界但不收敛
序列 $(-1)^n$ 有界，但不收敛。
:::

## 图表与兼容性

<!-- rc-label: fig:sample -->

![隔离练习图](assets/researchclip-eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee.png "练习图")

注释：图片与本段说明可以整体移动；练习PNG由测试工作区准备，不是真实研究数据。

<!-- rc-label: tab:parameters -->

| 参数 | 数值 | 注释 |
| :--- | ---: | :---: |
| $n$ | 2 | 中文 |
| 管道 | 1 | a\|b |

参见[定理](#rc-ref:thm:unique)、[图](#rc-ref:fig:sample)和[参数表](#rc-ref:tab:parameters)。

```latex
% 下列示例只是代码，不参与定理或公式编号。
\newcommand{\vect}[1]{\boldsymbol{#1}}
:::theorem 代码中的环境
fixture_text = "普通文本编辑器可读"
:::
```

自定义扩展保持原文：`{#custom-anchor}`、`[!custom]`。所有数学、注释和引用均可在普通文本编辑器阅读。
