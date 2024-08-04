# Targets
.PHONY: install
install:
	cargo install --path .
	nu -c "plugin add ~/.cargo/bin/nu_plugin_mssql"
	@echo "now run: 'plugin use mssql' in nushell"