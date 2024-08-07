let conn = mssql connect -i SQL2022 -t true -u cma -p cmacandc
$conn | mssql query -q "select * from sys.databases"