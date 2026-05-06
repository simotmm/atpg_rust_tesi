param(
    [Parameter(Mandatory=$true)][string]$RustOutputPath,
    [Parameter(Mandatory=$true)][string]$AtalantaReportPath,
    [Parameter(Mandatory=$true)][string]$AtalantaUndetectedPath
)

function Parse-AtalantaReport {
    param([string]$ReportPath)
    $map = @{}
    if (-not (Test-Path $ReportPath)) { return @{faults=0; d_faults=0; r_faults=0} }
    Get-Content $ReportPath | ForEach-Object {
        if ($_ -match '^([a-zA-Z0-9_]+):\s*(\S+)') {
            $key = $matches[1]
            $val = $matches[2]
            $map[$key] = $val
        }
    }
    $faults = 0
    $d_faults = 0
    $r_faults = 0
    if ($map.ContainsKey('faults')) { [int]::TryParse($map['faults'], [ref]$null) | Out-Null; $faults = [int]$map['faults'] }
    if ($map.ContainsKey('d_faults')) { $d_faults = [int]$map['d_faults'] }
    if ($map.ContainsKey('r_faults')) { $r_faults = [int]$map['r_faults'] }
    return @{faults = $faults; d_faults = $d_faults; r_faults = $r_faults}
}

function Parse-RustOutput {
    param([string]$RustPath)
    if (-not (Test-Path $RustPath)) { return $null }
    $content = Get-Content $RustPath -Raw
    $result = @{covered = $null; total = $null; pct = $null}
    $re = 'total faults covered.*?(\d+)\s*/\s*(\d+)\s*\(([0-9]+(?:\.[0-9]+)?)%\)'
    $m = [regex]::Match($content, $re, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
    if ($m.Success) {
        $result.covered = [int]$m.Groups[1].Value
        $result.total = [int]$m.Groups[2].Value
        $result.pct = [double]$m.Groups[3].Value
        return $result
    }
    # fallback: try to find any line containing 'total faults covered' and extract numbers by searching for X/Y (Z%)
    $lines = $content -split "\r?\n"
    foreach ($line in $lines) {
        if ($line -match 'total faults covered') {
            if ($line -match '(\d+)\s*/\s*(\d+)') { $result.covered = [int]$matches[1]; $result.total = [int]$matches[2] }
            if ($line -match '\(([0-9]+(?:\.[0-9]+)?)%\)') { $result.pct = [double]$matches[1] }
            if ($result.total -or $result.covered) { return $result }
        }
    }
    return $result
}

$atal = Parse-AtalantaReport -ReportPath $AtalantaReportPath
$at_total = $atal.faults
$at_detected = $atal.d_faults
$at_pct = 0
if ($at_total -gt 0) { $at_pct = [math]::Round(100 * ($at_detected / $at_total), 2) }

Write-Host "`n=== Combined Rust / Atalanta Summary ==="
Write-Host "Atalanta: total faults = $at_total; covered = $at_detected; coverage = $at_pct%"

$rust = Parse-RustOutput -RustPath $RustOutputPath
if ($null -eq $rust) {
    Write-Host "Rust: file output non trovato: $RustOutputPath"
} else {
    if ($rust.total -ne $null) { $rust_total = $rust.total } else { $rust_total = "?" }
    if ($rust.covered -ne $null) { $rust_covered = $rust.covered } else { $rust_covered = "?" }
    if ($rust.pct -ne $null) { $rust_pct = $rust.pct } else { $rust_pct = "?" }
    Write-Host "Rust: total faults = $rust_total; covered = $rust_covered; coverage = $rust_pct%"

    # Compare totals
    if (($rust_total -ne "?") -and ($at_total -ne 0)) {
        if ([int]$rust_total -eq [int]$at_total) {
            Write-Host "Totali: MATCH — stesso numero di fault: $rust_total"
        } elseif ([int]$rust_total -gt [int]$at_total) {
            $diff = [int]$rust_total - [int]$at_total
            Write-Host "Totali: Rust ha rilevato più fault di Atalanta: Rust=$rust_total vs Atalanta=$at_total (+$diff)"
        } else {
            $diff = [int]$at_total - [int]$rust_total
            Write-Host "Totali: Atalanta ha più fault registrati: Atalanta=$at_total vs Rust=$rust_total (+$diff)"
        }
    } else {
        Write-Host "Totals: comparison not possible (missing data)."
    }

    # Compare coperti
    if (($rust_covered -ne "?") -and ($at_detected -ne $null)) {
        if ([int]$rust_covered -eq [int]$at_detected) {
                Write-Host "Covered: MATCH — same number of covered faults: $rust_covered"
        } elseif ([int]$rust_covered -gt [int]$at_detected) {
                $diff = [int]$rust_covered - [int]$at_detected
                Write-Host "Covered: Rust covers more faults: Rust=$rust_covered vs Atalanta=$at_detected (+$diff)"
        } else {
                $diff = [int]$at_detected - [int]$rust_covered
                Write-Host "Covered: Atalanta covers more faults: Atalanta=$at_detected vs Rust=$rust_covered (+$diff)"
        }
        # coverage percentages comparison if available
            if ($rust_pct -ne "?") {
                Write-Host "Coverage percentages: Rust = $rust_pct% ; Atalanta = $at_pct%"
                if ([double]$rust_pct -gt [double]$at_pct) { Write-Host "Percentage: Rust shows higher coverage." }
                elseif ([double]$rust_pct -lt [double]$at_pct) { Write-Host "Percentage: Atalanta shows higher coverage." }
                else { Write-Host "Percentage: identical." }
            }
    } else {
            Write-Host "Covered: comparison not possible (missing data)."
    }
}

Write-Host "=========================`n"

exit 0
