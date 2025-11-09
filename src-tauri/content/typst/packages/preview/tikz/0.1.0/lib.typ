// Temporary TikZ placeholder package bundled for offline development.
// The real TikZ renderer should replace this module once the actual
// dependencies are mirrored into the Tideflow distribution.
#let render(
  diagram: str,
  scale: auto = none,
  preamble: str = "",
  format: str = "vector",
) = {
  let scale-label = if scale == auto {
    text(size: 8pt, fill: gray)[Scale: auto]
  } else {
    text(size: 8pt, fill: gray)[Scale: #str(scale)]
  }

  let format-label = if format == "" {
    none
  } else {
    text(size: 8pt, fill: gray)[Format: #format]
  }

  let preamble-block = if preamble.trim() == "" {
    none
  } else {
    block(
      spacing: 4pt,
    )[
      text(size: 8pt, weight: 600, fill: gray)[Preamble]
      box(fill: luma(255), inset: 6pt, stroke: none)[
        raw(preamble)
      ]
    ]
  }

  block(
    fill: luma(248),
    stroke: 0.5pt + luma(210),
    inset: 12pt,
    spacing: 8pt,
    radius: 8pt,
  )[
    text(size: 10pt, weight: 600, fill: gray)[TikZ preview placeholder]
    text(size: 8pt, fill: gray)[Replace with offline tikz renderer package.]
    scale-label
    if format-label != none {
      format-label
    }
    if preamble-block != none {
      preamble-block
    }
    box(width: 100%)[
      raw(diagram)
    ]
  ]
}
