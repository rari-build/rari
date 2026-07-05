$ErrorActionPreference = 'Stop'

$searchRoots = @(
  "$env:ChocolateyInstall\lib\redis",
  "$env:ChocolateyInstall\lib\redis-64",
  "C:\ProgramData\chocolatey\lib\redis",
  "C:\ProgramData\chocolatey\lib\redis-64"
) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -Unique

function Find-RedisExecutable {
  param([string[]] $Names)

  foreach ($name in $Names) {
    $command = Get-Command $name -ErrorAction SilentlyContinue
    if ($command) {
      return $command.Source
    }
  }

  foreach ($root in $searchRoots) {
    foreach ($name in $Names) {
      $file = Get-ChildItem -Path $root -Filter $name -Recurse -File -ErrorAction SilentlyContinue |
        Select-Object -First 1
      if ($file) {
        return $file.FullName
      }
    }
  }

  return $null
}

$redisService = Find-RedisExecutable @('RedisService.exe')
$redisServer = Find-RedisExecutable @('redis-server.exe', 'redis-server')
$redisCli = Find-RedisExecutable @('redis-cli.exe', 'redis-cli')

Write-Host "RedisService: $redisService"
Write-Host "redis-server: $redisServer"
Write-Host "redis-cli: $redisCli"

if (-not $redisCli) {
  throw 'redis-cli was not found'
}

if ($redisService) {
  Start-Process -FilePath $redisService -ArgumentList @('run', '--foreground', '--port', '6379') -WindowStyle Hidden
}
elseif ($redisServer) {
  Start-Process -FilePath $redisServer -ArgumentList @('--port', '6379') -WindowStyle Hidden
}
else {
  throw 'Redis server executable was not found'
}

$deadline = (Get-Date).AddSeconds(30)
do {
  & $redisCli -p 6379 ping
  if ($LASTEXITCODE -eq 0) {
    Write-Host 'Redis is ready'
    exit 0
  }
  Start-Sleep -Seconds 1
} while ((Get-Date) -lt $deadline)

throw 'Redis did not become ready'
