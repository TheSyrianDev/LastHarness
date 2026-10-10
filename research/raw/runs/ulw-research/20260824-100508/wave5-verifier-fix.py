import io

p = '.omo/ulw-research/20260822-143607/verify-research.ps1'
v = io.open(p, encoding='utf-8').read()
pairs = [
 ('    $expected = @(1..136 | Where-Object { $_ -ne 45 -and $_ -ne 38 })',
  '    $expected = @(1..146 | Where-Object { $_ -ne 45 -and $_ -ne 38 })'),
 ('-eq 135) { Pass "C3 extraction coverage: all 135 ledger entries present (132 active + merged/delisted history)" }',
  '-eq 145) { Pass "C3 extraction coverage: all 145 ledger entries present (142 active + merged/delisted history)" }'),
 ('Fail "C3 coverage: $($fxNums.Count)/135 rows',
  'Fail "C3 coverage: $($fxNums.Count)/145 rows'),
]
for o, n in pairs:
    assert o in v, o[:60]
    v = v.replace(o, n, 1)
if 'C7' not in v:
    L = []
    L.append('# --- C7: wave-4 dossier coverage (10 dossiers) ---')
    L.append('$dman4 = Join-Path $Root ".omo/ulw-research/20260824-100508/wave4-manifest.txt"')
    L.append('if (Test-Path $dman4) {')
    L.append('    $ok4=0; $bad4=@()')
    L.append('    Get-Content $dman4 | ForEach-Object {')
    L.append('        $f = Join-Path $Root ("research/surveys/10-feature-dossiers/wave4/" + $_ + ".md")')
    L.append("        if ((Test-Path $f) -and (Select-String -Path $f -Pattern 'Feature Dossier' -Quiet) -and (Select-String -Path $f -Pattern 'Per-feature verdict' -Quiet)) { $ok4++ } else { $bad4 += $_ }")
    L.append('    }')
    L.append('    if ($ok4 -eq 10 -and $bad4.Count -eq 0) { Pass "C7 wave-4 dossier coverage: all 10 present" }')
    L.append('    else { Fail ("C7 wave-4 dossiers: " + $ok4 + "/10 ok; bad: " + ($bad4 -join ", ")) }')
    L.append('} else { Fail ("wave-4 manifest missing: " + $dman4) }')
    L.append('')
    L.append('Write-Output "---"')
    v = v.replace('Write-Output "---"', '\n'.join(L), 1)
io.open(p, 'w', encoding='utf-8', newline='').write(v)

c = io.open('research/extraction/99-corrections.md', encoding='utf-8').read()
if 'Wave-5 trending' not in c:
    c += '''

## Wave-5 trending registrations (2026-08-25)

| Entry | Note |
|---|---|
| #137\u2013#146 | Ten GitHub-trending finds registered after API verification (stars/license/pushed all fresh). License flags: #137 NOASSERTION, #142 AGPL-3.0 (self-host OK / embed restrictive), #145 license-NULL. |
| #53 stat refresh | Hermes Agent now 236,302\u2605 (API 2026-08-25); genuine-growth trajectory confirmed. |
| #146 decision | QwenLM/qwen-code registered as distinct product (Alibaba terminal agent, Apache-2.0, 27.4k\u2605) \u2014 not adjacent to any registered Qwen ecosystem entry (MiMo Code #51 is Xiaomi). |
| Tier-B pre-dispositions | #143 ruflo + #144 9router registered WITH conflict expectations per triage; dossiers carry full teardown treatment. |
'''
io.open('research/extraction/99-corrections.md', 'w', encoding='utf-8', newline='').write(c)
print('verifier + corrections done')
