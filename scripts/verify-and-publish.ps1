# scripts/verify-and-publish.ps1
# Автоматическая верификация и публикация шага разработки smith-rust

param(
    [Parameter(Mandatory=$true)]
    [string]$Step,
    
    [Parameter(Mandatory=$true)]
    [string]$Message,
    
    [string]$Features = "",
    
    [switch]$NoPush  # Флаг для локального тестирования без пуша
)

$ErrorActionPreference = "Stop"
Write-Host "🚀 smith-rust: Верификация шага $Step" -ForegroundColor Cyan

# 1. Проверка формата
Write-Host "📐 Проверка форматирования..." -ForegroundColor Yellow
if (!(cargo fmt --check)) {
    Write-Host "❌ cargo fmt --check failed. Исправьте форматирование." -ForegroundColor Red
    exit 1
}

# 2. Проверка линтером
Write-Host "🔍 Запуск clippy..." -ForegroundColor Yellow
$clippyArgs = "clippy -- -D warnings"
if ($Features) { $clippyArgs = "--features $Features $clippyArgs" }
if (!(cargo $clippyArgs)) {
    Write-Host "❌ cargo clippy failed. Исправьте предупреждения." -ForegroundColor Red
    exit 1
}

# 3. Запуск тестов
Write-Host "🧪 Запуск тестов..." -ForegroundColor Yellow
$testArgs = "test"
if ($Features) { $testArgs = "--features $Features $testArgs" }
if (!(cargo $testArgs)) {
    Write-Host "❌ cargo test failed. Исправьте тесты." -ForegroundColor Red
    exit 1
}

# 4. Финальная сборка
Write-Host "📦 Финальная сборка..." -ForegroundColor Yellow
$buildArgs = "build"
if ($Features) { $buildArgs = "--features $Features $buildArgs" }
if (!(cargo $buildArgs)) {
    Write-Host "❌ cargo build failed." -ForegroundColor Red
    exit 1
}

# 5. Коммит (только если все проверки пройдены)
Write-Host "✅ Все проверки пройдены. Фиксация изменений..." -ForegroundColor Green
git add .
git commit -m "feat(step$Step): $Message"
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ git commit failed." -ForegroundColor Red
    exit 1
}

# 6. Push (если не указан флаг --NoPush)
if (!$NoPush) {
    Write-Host "📤 Отправка в удалённый репозиторий..." -ForegroundColor Green
    git push -u origin main
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ git push failed. Проверьте подключение и права." -ForegroundColor Red
        exit 1
    }
    Write-Host "🎉 Шаг $Step успешно опубликован!" -ForegroundColor Green
} else {
    Write-Host "⏭ Push пропущен (флаг --NoPush). Изменения закоммичены локально." -ForegroundColor Yellow
}