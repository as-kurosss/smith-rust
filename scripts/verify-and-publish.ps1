# scripts/verify-and-publish.ps1
# Версия 1.1: Исправлена работа в Windows/PowerShell

param(
    [Parameter(Mandatory=$true)]
    [string]$Step,
    
    [Parameter(Mandatory=$true)]
    [string]$Message,
    
    [string]$Features = "",
    
    [switch]$NoPush
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# 1. Переход в корень проекта
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Resolve-Path (Join-Path $ScriptDir "..")
Set-Location $ProjectRoot
Write-Host "🚀 smith-rust: Верификация шага $Step (в $ProjectRoot)" -ForegroundColor Cyan

# Вспомогательная функция для запуска cargo
function Invoke-Cargo {
    param([string[]]$Args)
    $proc = Start-Process "cargo" -ArgumentList $Args -NoNewWindow -Wait -PassThru
    if ($proc.ExitCode -ne 0) { return $false }
    return $true
}

# 2. Проверка форматирования
Write-Host "📐 Проверка форматирования..." -ForegroundColor Yellow
if (!(Invoke-Cargo "fmt", "--check")) {
    Write-Host "❌ cargo fmt --check failed" -ForegroundColor Red
    exit 1
}

# 3. Проверка линтером
Write-Host "🔍 Запуск clippy..." -ForegroundColor Yellow
$clippyArgs = @("clippy", "--", "-D", "warnings")
if ($Features) { $clippyArgs = @("--features", $Features) + $clippyArgs }
if (!(Invoke-Cargo $clippyArgs)) {
    Write-Host "❌ cargo clippy failed" -ForegroundColor Red
    exit 1
}

# 4. Запуск тестов
Write-Host "🧪 Запуск тестов..." -ForegroundColor Yellow
$testArgs = @("test")
if ($Features) { $testArgs = @("--features", $Features) + $testArgs }
if (!(Invoke-Cargo $testArgs)) {
    Write-Host "❌ cargo test failed" -ForegroundColor Red
    exit 1
}

# 5. Финальная сборка
Write-Host "📦 Финальная сборка..." -ForegroundColor Yellow
$buildArgs = @("build")
if ($Features) { $buildArgs = @("--features", $Features) + $buildArgs }
if (!(Invoke-Cargo $buildArgs)) {
    Write-Host "❌ cargo build failed" -ForegroundColor Red
    exit 1
}

# 6. Коммит
Write-Host "✅ Все проверки пройдены. Фиксация изменений..." -ForegroundColor Green
git add -A
$commitMsg = "feat(step$Step): $Message"
git commit -m $commitMsg
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ git commit failed" -ForegroundColor Red
    exit 1
}

# 7. Push
if (!$NoPush) {
    Write-Host "📤 Отправка в удалённый репозиторий..." -ForegroundColor Green
    git push -u origin main
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ git push failed" -ForegroundColor Red
        exit 1
    }
    Write-Host "🎉 Шаг $Step успешно опубликован!" -ForegroundColor Green
} else {
    Write-Host "⏭ Push пропущен (флаг --NoPush)" -ForegroundColor Yellow
}
