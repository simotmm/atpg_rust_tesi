param(
    [Parameter(Mandatory=$true)][int]$num
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $scriptDir "atpg_atlanta\atalanta-M\atalanta-M.exe"
$bench = Join-Path $scriptDir ("atpg_rust\benchmarks\converted_to_atlanta_iscas89\c{0}_to_iscas89_atlanta.bench" -f $num)
$inputTxt = Join-Path $scriptDir ("atpg_rust\results\c{0}_to_iscas89_atlanta_test_inputs.txt" -f $num)
$inputPat = Join-Path $scriptDir ("atpg_rust\results\c{0}_to_iscas89_atlanta_test_inputs.pat" -f $num)
$inputToUse = $null
$filtered = $null

if (-not (Test-Path $exe)) {
    Write-Error "Atalanta-M executable not found: $exe"
    exit 3
}
if (-not (Test-Path $bench)) {
    Write-Error "Bench file not found: $bench"
    exit 4
}

Write-Output "Running Atalanta-M on benchmark $num..."

# Use temporary files for Atalanta report and undetected list, remove after parsing
$report = Join-Path $env:TEMP ("atalanta_report_{0}.txt" -f $num)
$undetected = Join-Path $env:TEMP ("atalanta_undetected_{0}.txt" -f $num)

# workspace copy path (we will stream this file during execution and also save final copy)
$workspaceUndetected = Join-Path $scriptDir ("atpg_rust\results\c{0}_atalanta_undetected.txt" -f $num)

$args = @('-S','-P',$report,'-U',$undetected)

# Prefer a .pat file if available, otherwise try the .txt and filter only binary vectors
if (Test-Path $inputPat) {
    $inputToUse = $inputPat
} elseif (Test-Path $inputTxt) {
    $filtered = Join-Path $env:TEMP ("atalanta_vectors_{0}.pat" -f $num)
    Get-Content $inputTxt | Where-Object { $_ -match '^[01]+$' } | Set-Content -NoNewline $filtered
    if ((Test-Path $filtered) -and ((Get-Content $filtered).Length -gt 0)) {
        $inputToUse = $filtered
    } else {
        Remove-Item $filtered -ErrorAction SilentlyContinue
        $inputToUse = $null
    }
}

if ($inputToUse) { $args += '-t'; $args += $inputToUse }
$args += $bench

Write-Output '--- Atalanta-M output START (stdout/stderr) ---'

$exeDir = Split-Path -Parent $exe
Push-Location $exeDir

# Prepare temp files for stdout/stderr
$stdoutFile = Join-Path $env:TEMP ("atalanta_stdout_{0}.log" -f $num)
$stderrFile = Join-Path $env:TEMP ("atalanta_stderr_{0}.log" -f $num)
if (Test-Path $stdoutFile) { Remove-Item $stdoutFile -ErrorAction SilentlyContinue }
if (Test-Path $stderrFile) { Remove-Item $stderrFile -ErrorAction SilentlyContinue }

# Ensure temp undetected file is empty before starting (workspace file left alone)
if (Test-Path $undetected) { Remove-Item $undetected -ErrorAction SilentlyContinue }

# remove any old workspace file so streaming shows only current run's lines
if (Test-Path $workspaceUndetected) { Remove-Item $workspaceUndetected -ErrorAction SilentlyContinue }

# Start process and stream undetected file while running
$proc = Start-Process -FilePath $exe -ArgumentList $args -WorkingDirectory $exeDir -RedirectStandardOutput $stdoutFile -RedirectStandardError $stderrFile -PassThru

 $pos = 0
 $stdoutPos = 0
 $stderrPos = 0
while (-not $proc.HasExited) {
    # stream from workspace results file (Atalanta may be copying/updating it)
    if (Test-Path $workspaceUndetected) {
        $lines = Get-Content $workspaceUndetected -Raw -ErrorAction SilentlyContinue -Encoding UTF8 -Force | Out-String
        if ($lines) {
            $all = $lines -split "\r?\n"
            if ($all.Length -gt $pos) {
                $new = $all[$pos..($all.Length - 1)] | Where-Object { $_ -ne '' }
                $new | ForEach-Object { Write-Output $_ }
                $pos = $all.Length
            }
        }
    }
    # stream any new stdout lines produced by Atalanta
    if (Test-Path $stdoutFile) {
        $s = Get-Content $stdoutFile -Raw -ErrorAction SilentlyContinue -Encoding UTF8 -Force | Out-String
        if ($s) {
            $allS = $s -split "\r?\n"
            if ($allS.Length -gt $stdoutPos) {
                $newS = $allS[$stdoutPos..($allS.Length - 1)] | Where-Object { $_ -ne '' }
                $newS | ForEach-Object { Write-Output $_ }
                $stdoutPos = $allS.Length
            }
        }
    }
    if (Test-Path $stderrFile) {
        $e = Get-Content $stderrFile -Raw -ErrorAction SilentlyContinue -Encoding UTF8 -Force | Out-String
        if ($e) {
            $allE = $e -split "\r?\n"
            if ($allE.Length -gt $stderrPos) {
                $newE = $allE[$stderrPos..($allE.Length - 1)] | Where-Object { $_ -ne '' }
                $newE | ForEach-Object { Write-Output $_ }
                $stderrPos = $allE.Length
            }
        }
    }
    Start-Sleep -Milliseconds 200
}

# process exited - print any remaining undetected lines from workspace file
if (Test-Path $workspaceUndetected) {
    $lines = Get-Content $workspaceUndetected -Raw -ErrorAction SilentlyContinue -Encoding UTF8 -Force | Out-String
    if ($lines) {
        $all = $lines -split "\r?\n"
        if ($all.Length -gt $pos) {
            $new = $all[$pos..($all.Length - 1)] | Where-Object { $_ -ne '' }
            $new | ForEach-Object { Write-Output $_ }
            $pos = $all.Length
        }
    }
}

Pop-Location

# Dump captured stdout/stderr
if (Test-Path $stdoutFile) { Get-Content $stdoutFile | ForEach-Object { Write-Output $_ } }
if (Test-Path $stderrFile) { Get-Content $stderrFile | ForEach-Object { Write-Output $_ } }
Write-Output '--- Atalanta-M output END ---'

# If Atalanta produced an undetected list, display it and copy to workspace results
$workspaceUndetected = Join-Path $scriptDir ("atpg_rust\results\c{0}_atalanta_undetected.txt" -f $num)
if (Test-Path $undetected) {
    Copy-Item $undetected -Destination $workspaceUndetected -Force
    Write-Output ("-> Undetected faults saved to: " + $workspaceUndetected)
} else {
    Write-Output "-> No undetected-faults file produced by Atalanta."
}

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
        Write-Output "-> No fault information found in Atalanta report."
        exit 0
    }

    $miss = $faults - $detected
    $perc = [math]::Round(100 * ($detected / $faults), 2)

    if ($miss -eq 0) { Write-Output "-> all faults covered" } else { Write-Output ("-> not all faults covered (" + $miss + " undetected)") }
    Write-Output (" -> faults covered (Atalanta): " + $detected + "/" + $faults + " (" + $perc + "% )")
    Write-Output (" -> redundant faults: " + $redundant)

} finally {
    # cleanup temporary files
    if (Test-Path $report) { Remove-Item $report -ErrorAction SilentlyContinue }
    if (Test-Path $undetected) { Remove-Item $undetected -ErrorAction SilentlyContinue }
    if ($filtered -and (Test-Path $filtered)) { Remove-Item $filtered -ErrorAction SilentlyContinue }
}

exit 0
