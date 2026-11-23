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
# `figures-check` writes nothing and fails on a missing, stale or orphaned figure.

IXD ?= ixd

.PHONY: figures
figures:
	$(IXD) build docs/images/figures.json --out docs/images

.PHONY: figures-check
figures-check:
	$(IXD) build docs/images/figures.json --out docs/images --check
