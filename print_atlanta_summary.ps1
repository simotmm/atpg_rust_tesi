param(
    [Parameter(Mandatory=$true)][string]$ReportPath,
    [Parameter(Mandatory=$true)][string]$UndetectedPath
)

if (-not (Test-Path $ReportPath)) {
    Write-Host "Report file not found: $ReportPath"
    exit 2
}

$map = @{}
Get-Content $ReportPath | ForEach-Object {
    if ($_ -match '^(\w+):\s*(\S+)') { $map[$matches[1]] = $matches[2] }
}
$faults = if ($map.ContainsKey('faults')) { [int]$map['faults'] } else { 0 }
$d_faults = if ($map.ContainsKey('d_faults')) { [int]$map['d_faults'] } else { 0 }
$r_faults = if ($map.ContainsKey('r_faults')) { [int]$map['r_faults'] } else { 0 }
$coverage = if ($faults -gt 0) { [math]::Round(100 * ($d_faults / $faults), 2) } else { 0 }

Write-Host "`n=== Coverage Summary ==="
Write-Host "Total faults: $faults"
Write-Host "Detected faults: $d_faults"
Write-Host "Redundant faults: $r_faults"
Write-Host "Coverage: $coverage%"
if ($d_faults -ge $faults) {
    Write-Host "RESULT: OK — all faults covered by the provided input patterns."
} else {
    $miss = $faults - $d_faults
    Write-Host "RESULT: NOT OK — $miss faults NOT detected by the provided input patterns."
    if (Test-Path $UndetectedPath) {
        $undetected = Get-Content $UndetectedPath | Where-Object { $_ -and $_ -ne '' }
        if ($undetected.Count -gt 0) {
            Write-Host "Sample undetected faults (first 20):"
            $undetected | Select-Object -First 20 | ForEach-Object { Write-Host " - $_" }
        } else {
            Write-Host "Undetected file exists but is empty."
        }
    } else {
        Write-Host "No undetected file available to list specific faults."
    }
}
Write-Host "=========================`n"
