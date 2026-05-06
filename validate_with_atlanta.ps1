<#
validate_with_atlanta.ps1
Usage: .\validate_with_atlanta.ps1 <c-number>
Esempio: .\validate_with_atlanta.ps1 1355
#>
param(
    [Parameter(Mandatory=$true)]
    [string]$BenchmarkNumber
)

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Definition
$resultsDir = Join-Path $scriptRoot "atpg_rust\results"
$inputPattern = "c${BenchmarkNumber}_to_iscas89_atlanta_test_inputs.txt"
$inputFile = Join-Path $resultsDir $inputPattern

if (-not (Test-Path $inputFile)) {
    Write-Host "Input file not found: $inputFile"
    Write-Host "Cerco file parziali che contengono '$BenchmarkNumber' in $resultsDir"
    Get-ChildItem -Path $resultsDir -Filter "*${BenchmarkNumber}*" | ForEach-Object { Write-Host " - " $_.Name }
    exit 2
}

$exePath = Join-Path $scriptRoot "atpg_atlanta\atalanta-M\atalanta-M.exe"
if (-not (Test-Path $exePath)) {
    Write-Host "Eseguibile Atalanta non trovato in: $exePath"
    Write-Host "Verifica che `atpg_atlanta\atalanta-M\atalanta-M.exe` esista e sia eseguibile."
    exit 3
}

$outputFile = Join-Path $resultsDir ("validation_results_{0}.txt" -f $BenchmarkNumber)
$reportFile = Join-Path $resultsDir ("validation_report_{0}.txt" -f $BenchmarkNumber)
$undetectedFile = Join-Path $resultsDir ("validation_undetected_{0}.txt" -f $BenchmarkNumber)

Write-Host "Using input: $inputFile"
Write-Host "Atalanta exe: $exePath"
Write-Host "Saving results to: $outputFile"

# Start the process, feed the input file to its STDIN, capture stdout+stderr
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $exePath
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true

# Run Atalanta in simulation mode (-S) with pattern file (-t) and circuit name (c<bench>)
$atlantaDir = Join-Path $scriptRoot "atpg_atlanta\atalanta-M"
# prefer the converted .bench file in atpg_rust/benchmarks/converted_to_atlanta_iscas89
$benchDir = Join-Path $scriptRoot "atpg_rust\benchmarks\converted_to_atlanta_iscas89"
$benchFileCandidate = Join-Path $benchDir ("c{0}_to_iscas89_atlanta.bench" -f $BenchmarkNumber)
$benchFullPath = $null
if (Test-Path $benchFileCandidate) {
    $benchFullPath = (Get-Item $benchFileCandidate).FullName
} else {
    Write-Host "Bench file not found: $benchFileCandidate"
    Write-Host "Looked in: $benchDir"
    exit 5
}

# set Atalanta to run in simulation mode using the full .bench path
$psi.Arguments = "-S -t `"$inputFile`" -P `"$reportFile`" -U `"$undetectedFile`" `"$benchFullPath`""
$psi.WorkingDirectory = $atlantaDir

$proc = New-Object System.Diagnostics.Process
$proc.StartInfo = $psi

$started = $proc.Start()
if (-not $started) {
    Write-Host "Impossibile avviare Atalanta"
    exit 4
}

# Read all output and error
# capture stdout/stderr and wait
$out = $proc.StandardOutput.ReadToEnd()
$err = $proc.StandardError.ReadToEnd()
$proc.WaitForExit()

# Combine and save raw output
$combined = $out + "`n" + $err
Set-Content -Path $outputFile -Value $combined -NoNewline:$false

Write-Host "--- Begin Atalanta output (first 200 lines) ---"
$combined -split "\r?\n" | Select-Object -First 200 | ForEach-Object { Write-Host $_ }
Write-Host "--- End Atalanta output ---"
Write-Host "Full results saved to: $outputFile"

# Now analyze undetected faults file to determine coverage
if (Test-Path $undetectedFile) {
    $undetectedCount = (Get-Content $undetectedFile | Where-Object { $_ -and $_ -ne "" }).Count
    if ($undetectedCount -eq 0) {
        Write-Host "All faults detected by the provided input patterns. (undetected file empty)"
    } else {
        Write-Host "Detected faults: some faults were NOT detected. Undetected count: $undetectedCount"
        Write-Host "First 50 undetected faults:"
        Get-Content $undetectedFile | Select-Object -First 50 | ForEach-Object { Write-Host $_ }
    }
} else {
    Write-Host "Undetected faults file not produced: $undetectedFile"
    Write-Host "Attempting to parse report file for detection summary..."
    if (Test-Path $reportFile) {
        $r = Get-Content $reportFile
        $detectedLine = $r | Select-String -Pattern "detected" -SimpleMatch | Select-Object -First 1
        if ($detectedLine) { Write-Host "Report snippet:"; Write-Host $detectedLine.Line }
        else { Write-Host "No obvious 'detected' line found in report." }
    }
}

# Parse the report file and print a clear summary
if (Test-Path $reportFile) {
    $map = @{}
    Get-Content $reportFile | ForEach-Object {
        if ($_ -match '^(\w+):\s*(\S+)') { $map[$matches[1]] = $matches[2] }
    }
    $faults = [int]($map['faults'] -as [int])
    $d_faults = [int]($map['d_faults'] -as [int])
    $r_faults = [int]($map['r_faults'] -as [int])
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
        if (Test-Path $undetectedFile) {
            Write-Host "Sample undetected faults (first 20):"
            Get-Content $undetectedFile | Select-Object -First 20 | ForEach-Object { Write-Host " - $_" }
        } else {
            Write-Host "No undetected file available to list specific faults."
        }
    }
    Write-Host "=========================`n"
} else {
    Write-Host "Report file not found: $reportFile"
}
