# Measurements

Stage 4. The starting state, taken before the build, and what the build ended at.

| When | What | Value |
| --- | --- | --- |
| 2026-10-10, start | the folder | empty |
| 2026-10-10, first check | core tests, boundary tests | 87, 11 |
| 2026-10-10, first check | release wasm | 565 KB |
| 2026-10-10, first check | instances in an arena's static scene | 18,000 to 19,100 |
| 2026-10-10, first check | frames a second, headless Chromium on Metal, the title | 60 |
| 2026-10-10, after G7 | instances an arena (solid, ink) | about 4,000 |
| 2026-10-10, after G7 | frames a second on SwiftShader, the crater | 17 (was 5) |
| 2026-10-10, handoff | core tests, boundary tests | 88, 11 |
| 2026-11-08, the diver | the diver's download: mesh, animation, textures, manifest, clip table | 3.6 MB (mesh 0.99 MB, 0.65 MB gzipped; animation 2.06 MB, 0.69 MB gzipped; textures 0.66 MB; loaded after the title draws) |
| 2026-11-08, the diver | the diver | 26,000 triangles in 11 draws, 56 bones, 50 clips, 1,531 frames |
| 2026-11-08, the diver | release wasm (the clip table compiled in) | 2,258 KB |
| 2026-11-08, the diver | frame time in the Fleet (the Far Shore's first section), Chromium on Metal, 600 frames | median 16.70 ms, 95th 16.70 ms with the diver; 16.70 and 16.70 to 16.80 ms with the old figure (both held to the display's 60 Hz) |

