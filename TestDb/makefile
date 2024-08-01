# Makefile to spin up a test database in Docker

# Variables
DB_CONTAINER_NAME := sqlserver-pokemon
DB_HOSTNAME := sqlpreview
DB_PORT := 1433
DB_IMAGE := sqlserver-pokemon:latest
DB_PASSWORD := StrongP@ssw0rd123
DB_PID := Evaluation
ACCEPT_EULA := Y

.PHONY: build-db
build-db:
	@echo "Building SQL Server container..."
	docker build -t $(DB_IMAGE) -f dockerfile .

.PHONY: run-db
run-db:
	@echo "Starting SQL Server container..."
	docker run -e "ACCEPT_EULA=Y" -e "MSSQL_SA_PASSWORD=$(DB_PASSWORD)" -p 1433:1433 --name sqlserver-pokemon -d sqlserver-pokemon:latest

stop-db:
	@echo "Stopping SQL Server container..."
	docker stop $(DB_CONTAINER_NAME)

.PHONY: remove-db
remove-db:
	@echo "Removing SQL Server container..."
	docker rm $(DB_CONTAINER_NAME)

.PHONY: restart-db
restart-db: stop-db remove-db run-db

# Optional clean up target to stop and remove the container
.PHONY: clean
clean: stop-db remove-db
	@echo "Cleaned up SQL Server container."

# Optional status target to check the status of the container
.PHONY: status
status:
	docker ps -a | grep $(DB_CONTAINER_NAME)
