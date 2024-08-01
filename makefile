# Makefile to spin up a test database in Docker

# Variables
DB_CONTAINER_NAME := sqlpreview
DB_HOSTNAME := sqlpreview
DB_PORT := 1433
DB_IMAGE := mcr.microsoft.com/mssql/server:2022-preview-ubuntu-22.04
DB_PASSWORD := MyVeryStrongPassword1!
DB_PID := Evaluation
ACCEPT_EULA := Y

# Targets
.PHONY: install
install:
	cargo install --path .
	nu -c "plugin add ~/.cargo/bin/nu_plugin_mssql"
	nu -c "plugin use mssql"

.PHONY: run-db
run-db:
	@echo "Starting SQL Server container..."
	docker run -e "ACCEPT_EULA=$(ACCEPT_EULA)" \
	           -e "MSSQL_SA_PASSWORD=$(DB_PASSWORD)" \
	           -e "MSSQL_PID=$(DB_PID)" \
	           -p $(DB_PORT):1433 \
	           --name $(DB_CONTAINER_NAME) \
	           --hostname $(DB_HOSTNAME) \
	           -d $(DB_IMAGE)

.PHONY: stop-db
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
