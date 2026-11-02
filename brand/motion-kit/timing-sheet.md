# Timing sheet — "Appear and glance" (LastHarness mark)

60 fps. Comp 512 × 512 (64-unit grid × 8). Frames 0–84 play once (spawn); frames 84–492 loop.
Values are in grid units unless marked px. Easing names map to cubic-bezier curves in `tools/build_mark.py`:
`out` (0.22,1,0.36,1) · `inOut` (0.65,0,0.35,1) · `in` (0.55,0,1,0.45) · `outBack` (0.34,1.56,0.64,1) overshoot · `inBack` (0.36,0,0.66,-0.56) pull back.

## Story
He appears looking at you. A slot appears; he notices, turns and looks at it. The slot goes away; he looks for it for a beat, then turns back to you. Rule from the crew: **the work moves first, then the face looks.**

## Markers (in the Lottie file)
| Marker | Start | Frames | What happens |
|---|---|---|---|
| spawn | 0 | 84 | Face pops in, eyes open, first blink |
| loop | 84 | 408 | Everything below, loops seamlessly |
| slot-in | 126 | 33 | Slot pops in, ring burst, surprise |
| glance | 151 | 30 | Anticipation, then the turn to the slot |
| hold | 181 | 101 | Watching the slot, one blink |
| slot-out | 282 | 27 | Slot swells, shrinks to nothing, second ring |
| look-back | 294 | 66 | Searches past where it was, turns back to you, squash |
| idle | 360 | 132 | Looking at you, bobbing, double blink |

## Beats
| Frame | Part | Change | Ease into next |
|---|---|---|---|
| 0 | face | scale 0, opacity 0, eyes closed (scaleY 8%) | out |
| 8 | face | opacity 100 | — |
| 18 | face | scale 116 × 86 (squash on landing) | inOut |
| 28 | face | scale 92 × 110 (stretch) | inOut |
| 38 | face | scale 104 × 96 | inOut |
| 48 | face | scale 100 (settled) | — |
| 37 → 48 | eyes | open, scaleY 8% → 100% | outBack |
| 61 → 71 | eyes | blink (100 → 8 → 100) | in / out |
| 84 → 126 | face | bob up 4 px and back | inOut |
| 126 → 150 | slot | scale 0 → 100 with overshoot | outBack |
| 127 → 165 | ring | diameter 12 → 30, opacity 75 → 0 | out |
| 131 → 141 → 151 | face / eyes | surprise: face 97 × 105, eyes 112 × 118 | out / inOut |
| 151 → 159 | eyes | anticipation: moves −0.08 of the glance, rotate +0.64° | inOut |
| 159 → 181 | eyes | glance: translate (2.4, −2), rotate −8° about (27, 37) | inOut |
| 159 → 181 | far eye | 6 × 11 → 4.6 × 10.2, centre (31.4, 36.1) → (32.4, 36.5) | inOut |
| 159 → 181 | face | lean: rotate −3.5° about (27, 56), lift (+7, −6) px | inOut |
| 228 → 238 | eyes | blink | in / out |
| 282 → 290 | slot | swell 100 → 120 | out |
| 290 → 307 | slot | shrink 120 → 0 | inBack |
| 293 → 330 | ring | diameter 8 → 28, opacity 55 → 0 | out |
| 294 → 309 | eyes | overshoot: glance × 1.14 (looking for it) | out |
| 309 → 336 | eyes | turn back past centre to −0.14 | inOut |
| 309 → 336 | far eye | back to front shape | inOut |
| 294 → 330 | face | lean back to 0 | inOut |
| 336 → 354 | eyes | settle to 0 | inOut |
| 336 → 346 → 360 | face | squash 104 × 95 and back | out / inOut |
| 380 → 470 | face | two slow bobs, 4 px | inOut |
| 426, 439 | eyes | double blink | in / out |
| 492 | all | = frame 84 (seamless loop) | — |

## Hard rules
- The mark always fits the circle r30 around (32,32), except the ring burst and the spawn overshoot.
- Eyes never leave the face. The slot never touches the face (gap ≥ 3 even at 120%).
- The face never rotates more than 4°. The tilt lives in the eyes.
- Frame 84 and frame 492 must be identical.
