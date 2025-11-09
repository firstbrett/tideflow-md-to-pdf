# TikZ integration sample

```tikz scale=0.75 format="vector" preamble="\usetikzlibrary{calc}"
\begin{tikzpicture}
  \draw[step=1cm,gray,very thin] (-1.5,-1.5) grid (1.5,1.5);
  \draw[->,thick] (-1.5,0) -- (1.5,0);
  \draw[->,thick] (0,-1.5) -- (0,1.5);
  \draw[thick,blue] (0,0) circle (1cm);
\end{tikzpicture}
```

```tikz scale=auto format=png
% Diagrams can also omit a custom preamble and rely on defaults.
\begin{tikzpicture}
  \draw[thick,orange] (0,0) rectangle (2,1);
  \draw[->,orange] (0,0) -- (1,0.5);
\end{tikzpicture}
```

This file is intended for manual regression when developing the TikZ render
pipeline. The examples cover explicit scale, preamble, and format options.
