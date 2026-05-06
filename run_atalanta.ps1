param(
    [Parameter(Mandatory=$true)][int]$num
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $scriptDir "atpg_atlanta\atalanta-M\atalanta-M.exe"
$bench = Join-Path $scriptDir ("atpg_rust\benchmarks\converted_to_atlanta_iscas89\c{0}_to_iscas89_atlanta.bench" -f $num)
$input = Join-Path $scriptDir ("atpg_rust\results\c{0}_to_iscas89_atlanta_test_inputs.txt" -f $num)

if (-not (Test-Path $exe)) {
    Write-Error "Atalanta-M executable not found: $exe"
    exit 3
}
if (-not (Test-Path $bench)) {
    Write-Error "Bench file not found: $bench"
    exit 4
}

Write-Host "Running Atalanta-M on benchmark $num..."

# Use temporary files for Atalanta report and undetected list, remove after parsing
$report = Join-Path $env:TEMP ("atalanta_report_{0}.txt" -f $num)
$undetected = Join-Path $env:TEMP ("atalanta_undetected_{0}.txt" -f $num)

$args = @('-S','-P',$report,'-U',$undetected)
if (Test-Path $input) { $args += '-t'; $args += $input }
$args += $bench

Write-Host '--- Atalanta-M output START (stdout/stderr) ---'
$exeDir = Split-Path -Parent $exe
Push-Location $exeDir
try {
    $out = & $exe @args 2>&1
} finally {
    Pop-Location
}
$out | ForEach-Object { Write-Host $_ }
Write-Host '--- Atalanta-M output END ---'

# read and parse the generated report file (if produced)
$map = @{ 'faults' = 0; 'd_faults' = 0; 'r_faults' = 0 }
if (Test-Path $report) {
    Get-Content $report | ForEach-Object {
        if ($_ -match '^(\w+):\s*(\d+)') { $map[$matches[1]] = [int]$matches[2] }
    }
}
$faults = $map['faults']; $detected = $map['d_faults']; $redundant = $map['r_faults']

try {
    if ($faults -eq 0) {
        Write-Host "-> No fault information found in Atalanta report."
        exit 0
    }

    $miss = $faults - $detected
    $perc = [math]::Round(100 * ($detected / $faults), 2)

    if ($miss -eq 0) { Write-Host "-> all faults covered" } else { Write-Host ("-> not all faults covered (" + $miss + " undetected)") }
    Write-Host (" -> faults covered (Atalanta): " + $detected + "/" + $faults + " (" + $perc + "% )")
    Write-Host (" -> redundant faults: " + $redundant)

} finally {
    # cleanup temporary files
    if (Test-Path $report) { Remove-Item $report -ErrorAction SilentlyContinue }
    if (Test-Path $undetected) { Remove-Item $undetected -ErrorAction SilentlyContinue }
}

exit 0
