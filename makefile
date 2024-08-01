# Targets
.PHONY: install
install:
	cargo install --path .
	nu -c "plugin add ~/.cargo/bin/nu_plugin_mssql"
	nu -c "plugin use mssql"