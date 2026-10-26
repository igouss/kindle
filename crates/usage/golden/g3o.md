## What

Until Cutaway Street's open motion has ended and its size has settled, the plain street is drawn
without its hatching (the wall and earth `InkHatch`es) and without its street-life spans. Both are
computed once, when `drawn` flips, and ink in on the existing 450 ms OutCubic fade together with the
building art; a hatch's opacity follows `drawn`, not its area. `drawn` waits for both the settle
timer and `overview.motionSettled`. Everything else of the plain street shows from the first frame,
as today: walls (a flat tint with a 2 px outline when hatched, solid ink otherwise), ground lines,
labels, frontage dimensions, and the rooms with title, picture and selection frame.

## Force (evidence)

- After omarchy-expose-g3o.10, the frame gap holding the plain street's build in the first warm open
  is 143.7, 159.1 and 143.0 ms (g3o.10 builder comment, three interleaved live pairs, 13 windows on
  7 workspaces), and its heaviest QML line is still InkDrawing.js:21 (`hatch`). A kept item's area
  changes at every size the surface passes through, and InkHatch.qml:19 computes its lines again on
  every `area` change: the wall hatch (InkStreet.qml:336-342, `area: building.bounds`) and the earth
  hatch (InkStreet.qml:226-230, `area: ground.earth`). The g3o.10 verifier's probe: 12 hatch calls
  per resize, 0 per unchanged recompute.
- The spans (InkStreet.qml:250-273) are not under `drawn`; their `sheets` (Exterior.ground,
  InkStreet.qml:261) are computed again whenever `reach` changes, so at every passing size too.
- `drawn` (InkStreet.qml:87-99) is the 80 ms settle timer alone. `motionSettled` is
  `motionProgress >= 0.999` (OverviewState.qml:199); the default open motion is 190-400 ms by style
  (OverviewState.qml:60). Design answer 2 (epic notes, designer, 2026-10-25): with the per-size
  hatching gone the sizes settle sooner and the 80 ms timer would fire inside the open motion,
  landing the ink build on animated frames.
- AGENTS.md Profiling (the 2026-10-25 street bullet) names this as what removes the per-size
  hatching, pending what the street shows while it opens; design answer 2 settles that.

## What exists, what is missing

Exists: the `drawn` latch and InkArt's fade (InkArt.qml:41-44, an OpacityAnimator on a layer), which
already defer the building art; g3o.10's items kept across street recomputes. Missing: hatching and
spans computed only once `drawn` is true; a fade for them (the spans are bare InkSheets outside any
InkArt, and InkHatch has no opacity path); `drawn` waiting for the motion.

## How this differs from existing work

g3o.10 made each item be created once per open; the lines inside a kept hatch are still computed per
size. This bead removes that per-size computation. The rejected moves in the journal (memoizing
`hatch`, pattern-fill hatching) change how a hatch is computed, not how many times; g3o.11 was the
surface lifecycle.

## Measurement trap (cm-la6)

Waiting for `motionSettled` moves the ink build past "motion done" by construction, so the
motion-bounded sum of tests/profile-report (the "end by then" line, tests/profile-report:202-205)
falls even if no work were removed. The claim needs the first warm-open gap, both sums (to the close,
and end by then), and the gap that holds the ink build after motion done, on both trees.

## Open questions

- Responsiveness after the motion (operator, stop rule): the ink build now lands as the motion ends,
  when a keyboard user starts pressing arrows; a GUI stall there delays the first arrow. The gap that
  holds the ink build is measured; whether it is fast enough is the operator's rule ("fast enough
  for a user who likes a fast, responsive app", 2026-10-25), which names no number.
- Springy motion off (design, for the designer): the epic says a look with springy motion off opens
  in its finished state, and the 450 ms ink-in is Cutaway Street's opening moment (operator notes,
  2026-10-24). This bead puts the hatching and spans under that fade. Flagged assumption: with
  springy motion off they and the art appear at once when `drawn` flips, with no fade.
- Open motion of 0 ms (design, for the designer): `motionSettled` is true at once, so `drawn` falls
  back to the settle timer alone. Flagged assumption: that is intended.
- Verifier F3 on g3o.10 (operator): nothing in ./validate checks that a kept hatch follows its area
  (plant P4 caught only by a manual render comparison). This bead changes when that hatch is
  computed; whether a gate on drawing values is added is the operator's.

## Operator question

1. May this bead's assertions (nothing hatched and no span before `drawn`; `drawn` waits for the
   motion) run in ./validate, in tests/check-street-reuse or a sibling? No existing assertion
   changes; if one fails, that is a further question before any edit (cm-shm).
2. Is a lower motion-bounded sum with the ink build moved past motion done a win when the
   to-the-close sum does not fall?

## Operator decision

Inherited from omarchy-expose-g3o (2026-10-24, with the 2026-10-25 entries). That section approves
the six looks and the choices it lists, and nothing else. None beyond it: every policy question goes
back to the operator.

