> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

# PTX excerpt — `rx_pt_render_176` three-level loop skeleton (labels + branch structure)

Excerpted from `build/spike-rd027/bin/ctrl_b2.ptx` (3838 lines, PTX ISA 8.8, LLVM NVPTX /
clang 22.1.x; sha256 `85d597dd22e2d05f511a0cf8b2a27823bee78cf58523862eb06ccc67c738e315`) —
the single byte-identical artifact shared by all five run configurations. The file is a
workspace artifact (not committed); regeneration commands in `PROVENANCE.md`. **At file time
the full `.ptx` must be attached** — this excerpt (56 quoted lines) only shows the loop
skeleton the ptxas -O1+ latch restructuring acts on. Line numbers are 1-based file lines;
`N<TAB>text` per line. Line-ending note: the on-disk `.ptx` is CRLF (Windows toolchain
output); this excerpt is LF-normalized (repo is LF byte-exact), content otherwise verbatim
(verified byte-for-byte modulo the trailing CR, all 56 quoted lines).

Whole-file facts (static audit, spike report §4 / analysis §1.1): zero `.local`/spill; ~20
loops, all single-header natural loops, statically reducible; every counter has a unique
def-chain (init + single increment); all exits (except the cell inner loop) are
equality-compare exits; all multi-level breaks fold into the spp latch `$L__BB1_30`.

## Entry + virtual registers

```
150	.visible .entry rx_pt_render_176(
195		.reg .pred 	%p<285>;
196		.reg .b32 	%r<1004>;
197		.reg .b64 	%rd<62>;
```

## Runtime-derived DDA bound (%r6 = 3n+3)

```
270		mad.lo.s32 	%r6, %r120, 3, 3;
```

## Level 1 — spp loop: preheader, latch `$L__BB1_30`, header `$L__BB1_4`

```
294		mov.b32 	%r16, 0;
295		mov.b32 	%r153, 0f00000000;
296		setp.eq.b32 	%p9, %r119, 0;
297		setp.eq.b32 	%p55, %r6, 0;
298		mov.b32 	%r924, %r153;
299		mov.b32 	%r923, %r153;
300		mov.b32 	%r922, %r153;
301		bra.uni 	$L__BB1_4;
```

```
322	$L__BB1_30:                             // %bb25
323	                                        //   in Loop: Header=BB1_4 Depth=1
324		add.rn.f32 	%r924, %r924, %r27;
325		add.rn.f32 	%r923, %r923, %r28;
326		add.rn.f32 	%r922, %r922, %r29;
327		add.s32 	%r16, %r16, 1;
328		setp.eq.b32 	%p274, %r16, %r118;
329		@%p274 bra 	$L__BB1_31;
330	$L__BB1_4:                              // %bb10
331	                                        // =>This Loop Header: Depth=1
332	                                        //     Child Loop BB1_6 Depth 2
```

Nesting reaches depth 5 (LLVM loop annotations, sample):

```
346	                                        //       Child Loop BB1_116 Depth 3
347	                                        //         Child Loop BB1_120 Depth 4
348	                                        //           Child Loop BB1_123 Depth 5
```

## Level 2 — bounce loop: `bounces==0` fold, latch `$L__BB1_209`, header `$L__BB1_9`

```
443		@%p9 bra 	$L__BB1_30;
```

```
476	$L__BB1_209:                            // %bb129
477	                                        //   in Loop: Header=BB1_9 Depth=2
478		add.s32 	%r30, %r30, 1;
479		setp.eq.b32 	%p273, %r30, %r119;
480		@%p273 bra 	$L__BB1_30;
481	$L__BB1_9:                              // %bb24
482	                                        //   Parent Loop BB1_4 Depth=1
```

## Level 3 — main DDA march: preheader, latch `$L__BB1_67`, header `$L__BB1_44`

(This is the loop whose SASS count-exit becomes site 1 of the `@!P0 CALL.REL.NOINC`
restructuring at -O1: `aot_O1.sass` 2945 / `aot_O3.sass` 3664.)

```
753		mov.b32 	%r931, 0;
754		mov.b32 	%r932, -1;
755		mov.b32 	%r933, 0f7149F2CA;
756		bra.uni 	$L__BB1_44;
757	$L__BB1_67:                             // %bb86
758	                                        //   in Loop: Header=BB1_44 Depth=3
759		add.s32 	%r931, %r931, 1;
760		setp.eq.b32 	%p91, %r931, %r6;
761		@%p91 bra 	$L__BB1_54;
762	$L__BB1_44:                             // %bb61
763	                                        //   Parent Loop BB1_4 Depth=1
```

## Depth-4 cell/particle inner loop — the only less-than exit

```
799		add.s64 	%rd59, %rd59, 1;
800		setp.lt.u64 	%p72, %rd59, %rd5;
801		@%p72 bra 	$L__BB1_47;
802		bra.uni 	$L__BB1_60;
803	$L__BB1_47:                             // %bb67
804	                                        //   Parent Loop BB1_4 Depth=1
```

## Multi-level break fold: bounce-0 light-hit break -> spp latch; loop-nest exit

```
2373		add.rn.f32 	%r28, %r28, %r354;
2374		add.rn.f32 	%r29, %r29, %r355;
2375		bra.uni 	$L__BB1_30;
2376	$L__BB1_31:                             // %bb11
```

Every back edge above is closed by a compile-time constant or a runtime-parameter counter
bound; there is no construct in which a counter is updated differently on two branch arms.
The PTX-level conclusion (spike report §4): no semantically infinite path exists — the
deadlock is introduced downstream, by the ptxas -O1+/driver-JIT latch-exit restructuring
documented in `ISSUE_DRAFT.md`.
