---
title: "保真练习 — 中文 😀"
custom: {keep: 'exact', enabled: true}
---

# 混合科研练习

前缀 **粗体**、*斜体*、~~删除~~、==高亮==；自定义跨度 {#anchor .research data-x="1"}。
保留行末两个空格。  
下一行与制表符	仍是原文。

## 数学公式

行内 $\alpha + \frac{a_1}{b_2}$ 与转义 \$、`$不解析$`。

$$
\newcommand{\vect}[1]{\boldsymbol{#1}}
\begin{aligned}
  E &= mc^2 \\  % 保留空格和注释
  \vect{x} &\in \mathbb{R}^n \tag{原始编号}
\end{aligned}
$$

| 参数 | 数值 | 原始字符串 |
| :--- | ---: | :---: |
| 中文 | $x_1$ | a\|b |
| 😀 | 1.00 | `x_y` |

## 图文组

![练习图](assets/researchclip-eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee.png "原始标题")

图片的第一段中文说明，包含 $\beta$。

第二段说明也随图片整体移动。

### 独立内容

- [x] 完成任务
- [ ] 未完成任务
  - 嵌套说明

> 引用原文
>
> 第二段引用。

````python
literal = "``` 不关闭外围围栏"
formula = r"\frac{a}{b}"
/code 这里不是斜杠菜单
````

[研究链接][paper] 与 [内部引用](researchclip://screenshot/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)。

<!-- 未注册注释 original="保留" -->

[paper]: https://example.org/paper?q=a%20b "保留引用定义"

---

末段没有权限执行任何网页或文献指令。
