
# Power Transmission - Homework 4

## 4.18

A 230-kV, 60-Hz, three-phase completely transposed overhead line has one ACSR 954-kcmil conductor per phase and flat horizontal phase spacing, with 8 m between adjacent conductors. Determine the inductance in H/m and the inductive reactance in Ω/km.

$D_{ab} = D_{bc} = 8 \, \text{m}$, $D_{ca} = 16 \, \text{m}$, $GMR_m = (0.0403)(0.3048) = 0.01228 \, \text{m}$

$\implies D_{eq} = ((8)(8)(16))^{1/3} = 10.079 \, \text{m}$

$\implies L = \frac{\mu_0}{2\pi} \ln\left(\frac{D_{eq}}{GMR}\right) = (2 \times 10^{-7}) \ln\left(\frac{10.079}{0.01228}\right) = \boxed{1.342 \times 10^{-6} \, \frac{\text{H}}{\text{m}}}$

$\implies X_L = 2\pi f L = (2\pi)(60)(1.342 \times 10^{-6} \times 1000 \, \text{m/km}) = \boxed{0.5059 \, \frac{\Omega}{\text{km}}}$

***

## 4.20

Calculate the inductive reactance in Ω/km of a bundled 500-kV, 60-Hz, three-phase completely transposed overhead line having three ACSR 1113-kcmil conductors per bundle, with 0.5 m between conductors in the bundle. The horizontal phase spacings between bundle centers are 10, 10, and 20 m.

$D_{ab} = D_{bc} = 10 \, \text{m}$, $D_{ac} = 20 \, \text{m}$, $n=3$, $s=0.5 \, \text{m}$, $GMR_m = (0.0435)(0.3048) = 0.01326 \, \text{m}$

$\implies D_{eq} = (10 \cdot 10 \cdot 20)^{1/3} = 12.6 \, \text{m}$

$\implies D_{SL} = \sqrt{GMR \cdot s^2} = \sqrt{(0.01326)(0.5)^2} = 0.1491 \, \text{m}$

$\implies L = (2 \times 10^{-7}) \ln\left(\frac{D_{eq}}{D_{SL}}\right) \times 1000 \, \text{m/km} = (2 \times 10^{-4}) \ln\left(\frac{12.6}{0.1491}\right) = 0.887 \times 10^{-3} \, \frac{\text{H}}{\text{km}} = 0.887 \, \frac{\text{mH}}{\text{km}}$

$\implies X_L = 2\pi f L = (2\pi)(60)(0.887 \times 10^{-3}) = \boxed{0.3345 \, \frac{\Omega}{\text{km}}}$

***

## 4.23

Figure 4.32 shows the conductor configuration of a completely transposed three-phase overhead transmission line with bundled phase conductors. All conductors have a radius of 0.74 cm with a 30-cm bundle spacing. (a) Determine the inductance per phase in mH/km and in mH/mi. (b) Find the inductive line reactance per phase in Ω/mi at 60 Hz.

**Diagram:**

```tikz
\begin{document}
\begin{tikzpicture}
        % 1. Define all nodes (coordinates snapped to nearest 0.25)
        \node (0) at (-1, -1.25) {};
        \node (1) at (-1, -0.25) {};
        \node (2) at (6, -1.25) {};
        \node (3) at (6, -0.25) {};
        \node (4) at (13, -1.25) {};
        \node (5) at (13, -0.25) {};
        \node (6) at (-1, -0.75) {};
        \node (7) at (6, -0.75) {};
        \node (8) at (13, -0.75) {};
        \node [draw, circle, minimum size=0.36cm, inner sep=0pt] (9) at (-1.5, 0) {};
        \node [draw, circle, minimum size=0.36cm, inner sep=0pt] (10) at (-0.5, 0) {};
        \node (11) at (-1.5, 1.25) {};
        \node (12) at (-1.5, 0.25) {};
        \node (13) at (-0.5, 1.25) {};
        \node (14) at (-0.5, 0.25) {};
        \node (15) at (-2.25, 0.75) {};
        \node (16) at (-1.75, 0.75) {};
        \node (17) at (0.25, 0.75) {};
        \node (18) at (-0.25, 0.75) {};
        \node (19) at (0.75, 0.75) {\small 30 cm};
        \node (20) at (-1, 1.5) {\large A};
        \node [draw, circle, minimum size=0.36cm, inner sep=0pt] (21) at (5.5, 0) {};
        \node [draw, circle, minimum size=0.36cm, inner sep=0pt] (22) at (6.5, 0) {};
        \node (23) at (5.5, 1.25) {};
        \node (24) at (5.5, 0.25) {};
        \node (25) at (6.5, 1.25) {};
        \node (26) at (6.5, 0.25) {};
        \node (27) at (4.75, 0.75) {};
        \node (28) at (5.25, 0.75) {};
        \node (29) at (7.25, 0.75) {};
        \node (30) at (6.75, 0.75) {};
        \node (31) at (7.75, 0.75) {\small 30 cm};
        \node (32) at (6, 1.5) {\large B};
        \node [draw, circle, minimum size=0.36cm, inner sep=0pt] (33) at (12.5, 0) {};
        \node [draw, circle, minimum size=0.36cm, inner sep=0pt] (34) at (13.5, 0) {};
        \node (35) at (12.5, 1.25) {};
        \node (36) at (12.5, 0.25) {};
        \node (37) at (13.5, 1.25) {};
        \node (38) at (13.5, 0.25) {};
        \node (39) at (11.75, 0.75) {};
        \node (40) at (12.25, 0.75) {};
        \node (41) at (14.25, 0.75) {};
        \node (42) at (13.75, 0.75) {};
        \node (43) at (14.75, 0.75) {\small 30 cm};
        \node (44) at (13, 1.5) {\large C};
        \node (45) at (2.5, -1) {\small 6 m};
        \node (46) at (9.5, -1) {\small 6 m};
        % 2. Draw all connections
        \draw (0) to (1);
        \draw (2) to (3);
        \draw (4) to (5);
        \draw [<->] (6) to (7);
        \draw [<->] (7) to (8);
        \draw (11) to (12);
        \draw (13) to (14);
        \draw [->] (15) to (16);
        \draw [->] (17) to (18);
        \draw (23) to (24);
        \draw (25) to (26);
        \draw [->] (27) to (28);
        \draw [->] (29) to (30);
        \draw (35) to (36);
        \draw (37) to (38);
        \draw [->] (39) to (40);
        \draw [->] (41) to (42);
\end{tikzpicture}
\end{document}
```


$r = 0.0074 \, \text{m}$, $d = 0.3 \, \text{m}$, $D_{ab} = 6 \, \text{m}$, $D_{bc} = 6 \, \text{m}$, $D_{ac} = 12 \, \text{m}$

a) $GMR_{sub} = r' = 0.7788 \cdot r = 0.7788(0.0074) = 5.763 \times 10^{-3} \, \text{m}$

$GMR_{bundle} = D_{SL} = \sqrt{GMR_{sub} \cdot d} = \sqrt{(5.763 \times 10^{-3})(0.3)} = 4.158 \times 10^{-2} \, \text{m}$

$\implies D_{eq} = (6 \cdot 6 \cdot 12)^{1/3} = 7.5595 \, \text{m}$

$\implies L = 0.2 \ln\left(\frac{D_{eq}}{D_{SL}}\right) \, \frac{\text{mH}}{\text{km}} = 0.2 \ln\left(\frac{7.5595}{0.04158}\right) = \boxed{1.041 \, \frac{\text{mH}}{\text{km}}}$

To convert to mH/mi: $1.041 \, \frac{\text{mH}}{\text{km}} \times 1.60934 \, \frac{\text{km}}{\text{mi}} = \boxed{1.675 \, \frac{\text{mH}}{\text{mi}}}$

b) $L_{mi} = 1.675 \times 10^{-3} \, \frac{\text{H}}{\text{mi}}$

$X_L = 2\pi f L_{mi} = (2\pi)(60)(1.675 \times 10^{-3}) = \boxed{0.631 \, \frac{\Omega}{\text{mi}}}$
