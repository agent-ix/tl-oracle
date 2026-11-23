# =============================================================================
# TL Oracle Makefile
# =============================================================================

.PHONY: help
help:
	@echo "Available targets:"
	@echo "  make figures          - Regenerate docs/images with ixd-tl"
	@echo "  make figures-check    - Fail on a missing or stale figure, writes nothing"

# =============================================================================
# Documentation figures
# =============================================================================
#
# docs/images/figures.json declares the figures; the shared TL figures and the
# generate and check commands come from ix-diagrams. Install the tool once with
#   cargo install --locked --git https://github.com/agent-ix/ix-diagrams ixd-tl
# or point IXD_TL at a built binary. `figures-check` writes nothing and fails on
# a missing or stale SVG or a README image whose alt text differs from the claim.

IXD_TL ?= ixd-tl

.PHONY: figures
figures:
	$(IXD_TL) generate

.PHONY: figures-check
figures-check:
	$(IXD_TL) check
