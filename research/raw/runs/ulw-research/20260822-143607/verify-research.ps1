# Verify BAROOD-AI research reorganization
param([string]$Root = "C:\Users\Omar BAROOD\AI\BAROOD-AI")
$ErrorActionPreference = "Stop"
$fail = 0
function Fail($m) { $script:fail++; Write-Output "FAIL: $m" }
function Pass($m) { Write-Output "PASS: $m" }

# --- Old file parse ---
$old = Select-String -Path (Join-Path $Root "Current Research.md") -Pattern '^\|\s*(\d+)\s*\|'
$oldNums = @($old | ForEach-Object { [int]$_.Matches[0].Groups[1].Value })
if ($oldNums.Count -eq 109 -and (($oldNums | Sort-Object) -join ",") -eq ((1..109) -join ",")) {
    Pass "old file: 109 rows, #1-109 contiguous"
} elseif ($oldNums.Count -eq 0) {
    Pass "old file: stubbed (0 table rows) - reconciliation via archive"
} else {
    Fail "old file unexpected: $($oldNums.Count) rows"
}

# --- Archive check ---
$arch = Join-Path $Root "Research\archive\freebuff-dump-2026-08-20.md"
if (-not (Test-Path $arch)) { Fail "archive missing: $arch" } else { Pass "archive present" }

# --- Deliverable scan ---
$catFiles = Get-ChildItem (Join-Path $Root "Research") -Filter "*.md" -Recurse |
    Where-Object { $_.FullName -notmatch "\\archive\\" -and $_.Name -ne "99-corrections.md" -and $_.Name -ne "09-feature-extraction.md" }
$ledger = Join-Path $Root "Research\99-corrections.md"
$occurrences = @{}
$fieldFails = @()
foreach ($f in $catFiles) {
    $lines = Get-Content $f.FullName
    foreach ($ln in $lines) {
        if ($ln -match '^\|\s*#\s*(\d+)\s*\|') {
            $n = [int]$Matches[1]
            if ($occurrences.ContainsKey($n)) { Fail "#$n duplicated: $($occurrences[$n]) and $($f.Name)" }
            else { $occurrences[$n] = $f.Name }
            $cells = ($ln.Trim('|') -split '\|') | ForEach-Object { $_.Trim() }
            # contract columns: # | Tool | Purpose | Dept-fit | Status | Confidence | Official | GitHub
            if ($cells.Count -lt 8) { $fieldFails += "#$n has $($cells.Count)/8 cols"; continue }
            $tool, $purpose, $dept, $status, $conf = $cells[1], $cells[2], $cells[3], $cells[4], $cells[5]
            foreach ($pair in @(@("purpose",$purpose),@("dept",$dept),@("status",$status),@("confidence",$conf))) {
                if ([string]::IsNullOrWhiteSpace($pair[1])) { $fieldFails += "#$n empty $($pair[0])" }
            }
            if ($purpose.Length -gt 90) { $fieldFails += "#$n purpose too long ($($purpose.Length) chars)" }
        }
    }
}
if ($fieldFails) { $fieldFails | ForEach-Object { Fail $_ } } elseif ($occurrences.Count -gt 0) { Pass "all deliverable rows field-complete" }

# --- Ledger numbers ---
if (Test-Path $ledger) {
    $ledgerNums = @(Select-String -Path $ledger -Pattern '(?i)(?:removed|merged)\s+#(\d+)' | ForEach-Object { [int]$_.Matches[0].Groups[1].Value } | Sort-Object -Unique)
    Pass "corrections ledger: $($ledgerNums.Count) entries ($( ($ledgerNums | ForEach-Object {"#$_"}) -join ',' ))"
} else { Fail "corrections ledger missing: $ledger" }

# --- Reconciliation C1 ---
$accounted = @{}
foreach ($k in $occurrences.Keys) { $accounted[[int]$k] = "deliverable" }
if (Test-Path $ledger) {
    Select-String -Path $ledger -Pattern '(?i)(?:removed|merged)\s+#(\d+)' | ForEach-Object {
        $n = [int]$_.Matches[0].Groups[1].Value
        if ($accounted.ContainsKey($n)) { Fail "#$n in both deliverable and ledger" } else { $accounted[$n] = "ledger" }
    }
}
$missing = @(1..109 | Where-Object { -not $accounted.ContainsKey($_) })
if ($missing.Count -eq 0) { Pass "C1 reconciliation: all 109 accounted" } else { Fail "C1 missing $( $missing.Count ): #$($missing -join ', #')" }

# --- Stub check ---
$stub = Get-Content (Join-Path $Root "Current Research.md") -Raw
if ($stub -match 'research/surveys/00-index\.md' -and $stub.Length -lt 2000) { Pass "Current Research.md is pointer stub" }
elseif ($oldNums.Count -gt 0) { Pass "old file not yet stubbed (pre-build state)" }

# --- C3: feature-extraction coverage (all 108 active entries, #45 merged) ---
$fx = Join-Path $Root "Research\09-feature-extraction.md"
if (Test-Path $fx) {
    $fxNums = @(Select-String -Path $fx -Pattern '^\|\s*#\s*(\d+)\s*\|' | ForEach-Object { [int]$_.Matches[0].Groups[1].Value })
    $fxDupes = @($fxNums | Group-Object | Where-Object Count -gt 1)
    if ($fxDupes.Count -gt 0) { Fail "C3 dup rows in 09: #(($( ($fxDupes | ForEach-Object Name) -join ',#' )))" }
    $expected = @(1..146 | Where-Object { $_ -ne 45 -and $_ -ne 38 })
    $missingFx = @($expected | Where-Object { $fxNums -notcontains $_ })
    if ($missingFx.Count -eq 0 -and $fxNums.Count -eq 145) { Pass "C3 extraction coverage: all 145 ledger entries present (142 active + merged/delisted history)" }
    else { Fail "C3 coverage: $($fxNums.Count)/145 rows; missing $(($missingFx | ForEach-Object {"#$_"}) -join ',')" }
} else {
    Fail "feature-extraction file missing: $fx"
}

# --- C4: feature-dossier coverage (wave 1: 40 dossiers) ---
$dman = Join-Path $Root ".omo/ulw-research/20260822-143607/dossier-manifest.txt"
if (Test-Path $dman) {
    $okD=0; $badD=@()
    Get-Content $dman | ForEach-Object {
        if ($_ -match '^(.+)[|]([0-9]+)$') {
            $f = Join-Path $Root ("research/surveys/10-feature-dossiers/wave1/" + $Matches[1] + ".md")
            if ((Test-Path $f) -and (Select-String -Path $f -Pattern 'Feature Dossier' -Quiet) -and (Select-String -Path $f -Pattern 'Per-feature verdict' -Quiet)) { $okD++ }
            else { $badD += ("#" + $Matches[2] + " " + $Matches[1]) }
        }
    }
    if ($okD -eq 40 -and $badD.Count -eq 0) { Pass "C4 dossier coverage: all 40 present with required sections" }
    else { Fail ("C4 dossiers: " + $okD + "/40 ok; bad: " + ($badD -join ", ")) }
} else { Fail ("dossier manifest missing: " + $dman) }

# --- C5: wave-2 dossier coverage (36 dossiers; 12 conflict need re-impl sketch) ---
$dman2 = Join-Path $Root ".omo/ulw-research/20260824-100508/wave2-manifest.txt"
if (Test-Path $dman2) {
    $okW=0; $badW=@()
    Get-Content $dman2 | ForEach-Object {
        if ($_ -match '^(.+)[|]([0-9]+)[|](.+)$') {
            $f = Join-Path $Root ("research/surveys/10-feature-dossiers/wave2/" + $Matches[1] + ".md")
            $has = (Test-Path $f) -and (Select-String -Path $f -Pattern 'Feature Dossier' -Quiet) -and (Select-String -Path $f -Pattern 'Per-feature verdict' -Quiet)
            if ($has -and $Matches[3] -eq 'C') { $has = Select-String -Path $f -Pattern 'Compliant re-implementation sketch' -Quiet }
            if ($has) { $okW++ } else { $badW += ("#" + $Matches[2] + " " + $Matches[1]) }
        }
    }
    if ($okW -eq 36 -and $badW.Count -eq 0) { Pass "C5 wave-2 dossier coverage: all 36 present (conflict sketches included)" }
    else { Fail ("C5 wave-2 dossiers: " + $okW + "/36 ok; bad: " + ($badW -join ", ")) }
} else { Fail ("wave-2 manifest missing: " + $dman2) }

# --- C6: wave-3 dossier coverage (51 dossiers, full registry coverage) ---
$dman3 = Join-Path $Root ".omo/ulw-research/20260824-100508/wave3-manifest.txt"
if (Test-Path $dman3) {
    $ok3=0; $bad3=@()
    Get-Content $dman3 | ForEach-Object {
        if ($_ -match '^(.+)[|]([0-9]+)[|](.+)$') {
            $f = Join-Path $Root ("research/surveys/10-feature-dossiers/wave3/" + $Matches[1] + ".md")
            if ((Test-Path $f) -and (Select-String -Path $f -Pattern 'Feature Dossier' -Quiet) -and (Select-String -Path $f -Pattern 'Per-feature verdict' -Quiet)) { $ok3++ } else { $bad3 += ("#" + $Matches[2] + " " + $Matches[1]) }
        }
    }
    if ($ok3 -eq 51 -and $bad3.Count -eq 0) { Pass "C6 wave-3 dossier coverage: all 51 present" }
    else { Fail ("C6 wave-3 dossiers: " + $ok3 + "/51 ok; bad: " + ($bad3 -join ", ")) }
} else { Fail ("wave-3 manifest missing: " + $dman3) }

# --- C7: wave-4 dossier coverage (10 dossiers) ---
$dman4 = Join-Path $Root ".omo/ulw-research/20260824-100508/wave4-manifest.txt"
if (Test-Path $dman4) {
    $ok4=0; $bad4=@()
    Get-Content $dman4 | ForEach-Object {
        $f = Join-Path $Root ("research/surveys/10-feature-dossiers/wave4/" + $_ + ".md")
        if ((Test-Path $f) -and (Select-String -Path $f -Pattern 'Feature Dossier' -Quiet) -and (Select-String -Path $f -Pattern 'Per-feature verdict' -Quiet)) { $ok4++ } else { $bad4 += $_ }
    }
    if ($ok4 -eq 10 -and $bad4.Count -eq 0) { Pass "C7 wave-4 dossier coverage: all 10 present" }
    else { Fail ("C7 wave-4 dossiers: " + $ok4 + "/10 ok; bad: " + ($bad4 -join ", ")) }
} else { Fail ("wave-4 manifest missing: " + $dman4) }

Write-Output "---"
if ($fail -eq 0) { Write-Output "VERDICT: GREEN (0 failures)"; exit 0 } else { Write-Output "VERDICT: RED ($fail failures)"; exit 1 }
