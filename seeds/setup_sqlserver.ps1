# Enable TCP/IP for SQL Server Express via registry
$regBase = 'HKLM:\SOFTWARE\Microsoft\Microsoft SQL Server'

# Find the instance path
$instanceName = (Get-ItemProperty "$regBase\Instance Names\SQL").SQLEXPRESS
Write-Host "Instance: $instanceName"

# Enable TCP/IP via registry
$netPath = "$regBase\$instanceName\MSSQLServer\SuperSocketNetLib\Tcp"
Set-ItemProperty -Path $netPath -Name 'Enabled' -Value 1
Write-Host 'TCP/IP enabled via registry'

# Set port 1433 on IPAll
$ipAllPath = "$netPath\IPAll"
Set-ItemProperty -Path $ipAllPath -Name 'TcpPort' -Value '1433'
Set-ItemProperty -Path $ipAllPath -Name 'TcpDynamicPorts' -Value ''
Write-Host 'Port 1433 set on IPAll'

# Enable mixed mode authentication
$mssqlPath = "$regBase\$instanceName\MSSQLServer"
Set-ItemProperty -Path $mssqlPath -Name 'LoginMode' -Value 2
Write-Host 'Mixed mode authentication enabled'

# Restart SQL Server
Restart-Service 'MSSQL$SQLEXPRESS' -Force
Start-Sleep -Seconds 3
Write-Host 'SQL Server restarted'

# Install SqlServer module for Invoke-Sqlcmd
Install-Module -Name SqlServer -Force -AllowClobber -Scope CurrentUser
Import-Module SqlServer
Write-Host 'SqlServer module installed'

# Enable SA login
Invoke-Sqlcmd -ServerInstance 'localhost\SQLEXPRESS' -Query "ALTER LOGIN sa ENABLE; ALTER LOGIN sa WITH PASSWORD = 'YourPassword123';" -TrustServerCertificate
Write-Host 'SA login enabled'

# Restart once more for good measure
Restart-Service 'MSSQL$SQLEXPRESS' -Force
Start-Sleep -Seconds 3

# Seed the database
Invoke-Sqlcmd -ServerInstance 'localhost,1433' -Username 'sa' -Password 'YourPassword123' -InputFile 'C:\Users\rolan\Desktop\Upsert\seeds\sqlserver_seed.sql' -TrustServerCertificate
Write-Host 'Database seeded successfully!'
