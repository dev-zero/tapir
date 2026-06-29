# Font build system: fetch BDF sources, verify SHA-256.
# Requires: curl, sha256sum/shasum, unzip
#
# Usage:
#   make fonts          - fetch all
#   make clean-fonts    - remove generated files

CURL       ?= curl -fsSL

FONT_DIR := fonts/otb

# Use sha256sum (Linux) or shasum (macOS)
SHA256 = $(if $(shell command -v sha256sum 2>/dev/null),sha256sum,shasum -a 256)

# ─── Font sources ────────────────────────────────────────────────────────────

ADOBE_100DPI_VERSION := 1.0.4
ADOBE_100DPI_URL     := https://xorg.freedesktop.org/releases/individual/font/font-adobe-100dpi-$(ADOBE_100DPI_VERSION).tar.xz
ADOBE_100DPI_SHA256  := b67aff445e056328d53f9732d39884f55dd8d303fc25af3dbba33a8ba35a9ccf

TERMINUS_VERSION := 4.49.1
TERMINUS_URL     := https://downloads.sourceforge.net/project/terminus-font/terminus-font-4.49/terminus-font-$(TERMINUS_VERSION).tar.gz
TERMINUS_SHA256  := d961c1b781627bf417f9b340693d64fc219e0113ad3a3af1a3424c7aa373ef79

SPLEEN_VERSION := 2.1.0
SPLEEN_URL     := https://github.com/fcambus/spleen/releases/download/$(SPLEEN_VERSION)/spleen-$(SPLEEN_VERSION).tar.gz
SPLEEN_SHA256  := 8b47c56f1a6eb858fbcf9e34530557404b02fbb3455e38e64fb84473fd0c372f

THERMAL_VERSION := 0.2
THERMAL_URL     := https://github.com/mike42/thermal-sans-mono/releases/download/v$(THERMAL_VERSION)/thermal-sans-mono-v$(THERMAL_VERSION).tar.gz
THERMAL_SHA256  := 6c5d9d92c8e362eb7cc1d763f0c4c6c2038ac5f4d20a2a35a39619adb0783121

TINY5_VERSION := 1.002
TINY5_URL     := https://github.com/Gissio/font_Tiny5/archive/refs/tags/$(TINY5_VERSION).tar.gz
TINY5_SHA256  := 7f78949b0f83e0053b7ad620962ef4e7f51384d9a618fc4f0d09e1f59ee14aad

PROFONT_URL    := https://tobiasjung.name/downloadfile.php?file=profont-otb-2.zip
PROFONT_SHA256 := 2f8437a7cfef20babbb78b0ff90a269e8f58f2b9595670e226c20ea7c140bae5

# ─── Top-level targets ───────────────────────────────────────────────────────

.PHONY: fonts clean-fonts

fonts: $(FONT_DIR)/.adobe-100dpi $(FONT_DIR)/.terminus $(FONT_DIR)/.spleen $(FONT_DIR)/.thermal $(FONT_DIR)/.tiny5 $(FONT_DIR)/.profont

# ─── Fetch + extract rules ───────────────────────────────────────────────────

$(FONT_DIR):
	mkdir -p $@

$(FONT_DIR)/.adobe-100dpi: | $(FONT_DIR)
	@echo "Fetching Adobe 100dpi fonts..."
	@$(CURL) -o /tmp/font-adobe-100dpi.tar.xz "$(ADOBE_100DPI_URL)"
	@echo "$(ADOBE_100DPI_SHA256)  /tmp/font-adobe-100dpi.tar.xz" | $(SHA256) -c -
	@tar -xJf /tmp/font-adobe-100dpi.tar.xz -C /tmp/
	@cd /tmp/font-adobe-100dpi-$(ADOBE_100DPI_VERSION) && \
		cp helvR08.bdf helvR10.bdf helvR12.bdf helvR14.bdf helvR18.bdf helvR24.bdf \
		   helvB08.bdf helvB10.bdf helvB12.bdf helvB14.bdf helvB18.bdf helvB24.bdf \
		   helvO08.bdf helvO10.bdf helvO12.bdf helvO14.bdf helvO18.bdf helvO24.bdf \
		   helvBO08.bdf helvBO10.bdf helvBO12.bdf helvBO14.bdf helvBO18.bdf helvBO24.bdf \
		   timR08.bdf timR10.bdf timR12.bdf timR14.bdf timR18.bdf timR24.bdf \
		   timB08.bdf timB10.bdf timB12.bdf timB14.bdf timB18.bdf timB24.bdf \
		   timI08.bdf timI10.bdf timI12.bdf timI14.bdf timI18.bdf timI24.bdf \
		   timBI08.bdf timBI10.bdf timBI12.bdf timBI14.bdf timBI18.bdf timBI24.bdf \
		   $(CURDIR)/$(FONT_DIR)/
	@rm -rf /tmp/font-adobe-100dpi-$(ADOBE_100DPI_VERSION) /tmp/font-adobe-100dpi.tar.xz
	@touch $@

$(FONT_DIR)/.terminus: | $(FONT_DIR)
	@echo "Fetching Terminus $(TERMINUS_VERSION)..."
	@$(CURL) -o /tmp/terminus-font.tar.gz "$(TERMINUS_URL)"
	@echo "$(TERMINUS_SHA256)  /tmp/terminus-font.tar.gz" | $(SHA256) -c -
	@tar -xzf /tmp/terminus-font.tar.gz -C /tmp/
	@cp /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u12n.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u14n.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u16n.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u18n.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u20n.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u22n.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u24n.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u28n.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u32n.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u12b.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u14b.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u16b.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u18b.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u20b.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u22b.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u24b.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u28b.bdf \
	    /tmp/terminus-font-$(TERMINUS_VERSION)/ter-u32b.bdf \
	    $(FONT_DIR)/
	@rm -rf /tmp/terminus-font-$(TERMINUS_VERSION) /tmp/terminus-font.tar.gz
	@touch $@

$(FONT_DIR)/.spleen: | $(FONT_DIR)
	@echo "Fetching Spleen $(SPLEEN_VERSION)..."
	@$(CURL) -o /tmp/spleen.tar.gz "$(SPLEEN_URL)"
	@echo "$(SPLEEN_SHA256)  /tmp/spleen.tar.gz" | $(SHA256) -c -
	@tar -xzf /tmp/spleen.tar.gz -C /tmp/
	@cp /tmp/spleen-$(SPLEEN_VERSION)/spleen-5x8.bdf \
	    /tmp/spleen-$(SPLEEN_VERSION)/spleen-6x12.bdf \
	    /tmp/spleen-$(SPLEEN_VERSION)/spleen-8x16.bdf \
	    /tmp/spleen-$(SPLEEN_VERSION)/spleen-12x24.bdf \
	    /tmp/spleen-$(SPLEEN_VERSION)/spleen-16x32.bdf \
	    /tmp/spleen-$(SPLEEN_VERSION)/spleen-32x64.bdf \
	    $(FONT_DIR)/
	@rm -rf /tmp/spleen-$(SPLEEN_VERSION) /tmp/spleen.tar.gz
	@touch $@

$(FONT_DIR)/.thermal: | $(FONT_DIR)
	@echo "Fetching Thermal Sans Mono v$(THERMAL_VERSION)..."
	@$(CURL) -o /tmp/thermal-sans-mono.tar.gz "$(THERMAL_URL)"
	@echo "$(THERMAL_SHA256)  /tmp/thermal-sans-mono.tar.gz" | $(SHA256) -c -
	@tar -xzf /tmp/thermal-sans-mono.tar.gz -C /tmp/
	@cp /tmp/thermal-sans-mono/thermal-sans-mono-17/thermal-sans-mono-17.bdf \
	    /tmp/thermal-sans-mono/thermal-sans-mono-24/thermal-sans-mono-24.bdf \
	    $(FONT_DIR)/
	@rm -rf /tmp/thermal-sans-mono /tmp/thermal-sans-mono.tar.gz
	@touch $@

$(FONT_DIR)/.tiny5: | $(FONT_DIR)
	@echo "Fetching Tiny5 $(TINY5_VERSION)..."
	@$(CURL) -o /tmp/font_Tiny5.tar.gz "$(TINY5_URL)"
	@echo "$(TINY5_SHA256)  /tmp/font_Tiny5.tar.gz" | $(SHA256) -c -
	@tar -xzf /tmp/font_Tiny5.tar.gz -C /tmp/
	@cp /tmp/font_Tiny5-$(TINY5_VERSION)/fonts/bdf/tiny5-Regular.bdf $(FONT_DIR)/
	@rm -rf /tmp/font_Tiny5-$(TINY5_VERSION) /tmp/font_Tiny5.tar.gz
	@touch $@

$(FONT_DIR)/.profont: | $(FONT_DIR)
	@echo "Fetching ProFont OTB..."
	@$(CURL) -o /tmp/profont-otb-2.zip "$(PROFONT_URL)"
	@echo "$(PROFONT_SHA256)  /tmp/profont-otb-2.zip" | $(SHA256) -c -
	@unzip -qo /tmp/profont-otb-2.zip -d /tmp/
	@cp /tmp/profont-otb-2/ProFontOTB.otb $(FONT_DIR)/profont-regular.otb
	@chmod 644 $(FONT_DIR)/profont-regular.otb
	@rm -rf /tmp/profont-otb-2 /tmp/profont-otb-2.zip
	@touch $@

# ─── Clean ───────────────────────────────────────────────────────────────────

clean-fonts:
	rm -rf $(FONT_DIR)
