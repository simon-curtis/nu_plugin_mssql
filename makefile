UNAME := $(shell uname)
PLUGIN_NAME := nu_plugin_mssql

# Check if the operating system is Windows (using MINGW as an example)
ifeq ($(OS), Windows_NT)
    PLUGIN_NAME := nu_plugin_mssql.exe
else
    # Further checks can be added for other platforms if needed
    ifeq ($(UNAME), Linux)
        # Linux-specific settings (if any)
    else ifeq ($(UNAME), Darwin)
        # macOS-specific settings (if any)
    endif
endif

# Targets
.PHONY: install
install:
	cargo install --path .
	nu -c "plugin add ~/.cargo/bin/$(PLUGIN_NAME)"
	@echo "now run: 'plugin use mssql' in nushell"