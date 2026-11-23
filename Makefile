# =============================================================================
# TL Oracle Makefile
# =============================================================================

.PHONY: help
help:
	@echo "Available targets:"
	@echo "  make figures          - Regenerate docs/images with ixd build"
	@echo "  make figures-check    - Fail on a missing or stale figure, writes nothing"

# =============================================================================
# Documentation figures
# =============================================================================
#
# docs/images/figures.json declares the figures and is built by the ix-diagrams `ixd`
# command (cargo install --locked --git https://github.com/agent-ix/ix-diagrams ix-diagrams).
# `figures-check` writes nothing and fails on a missing, stale or orphaned figure. It is the local
# pre-merge regeneration check; hosted CI is manual-only and does not run it.

IXD ?= ixd

ifneq ($(filter figures figures-check,$(MAKECMDGOALS)),)
ifeq ($(shell command -v $(IXD) 2>/dev/null),)
$(error ixd not found: install with cargo install --locked --git https://github.com/agent-ix/ix-diagrams ix-diagrams or set IXD=<path>)
endif
endif

.PHONY: figures
figures:
	$(IXD) build docs/images/figures.json --out docs/images

.PHONY: figures-check
figures-check:
	$(IXD) build docs/images/figures.json --out docs/images --check
