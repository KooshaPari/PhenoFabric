.PHONY: bundle bundle-macos bundle-linux bundle-windows install clean help

BINARY_NAME := fabric-gui
RELEASE_BIN := target/release/$(BINARY_NAME)

# Auto-detect platform
UNAME_S := $(shell uname -s)

## bundle: Auto-detect platform and build app bundle
bundle:
	@case "$(UNAME_S)" in \
		Darwin)  $(MAKE) bundle-macos ;; \
		Linux)  $(MAKE) bundle-linux ;; \
		*)      echo "Unsupported platform: $(UNAME_S)"; exit 1 ;; \
	esac

## bundle-macos: Build macOS .app bundle
bundle-macos: $(RELEASE_BIN)
	@echo "==> Building macOS app bundle..."
	bash bundle/macos/build-app.sh

## bundle-linux: Create Linux package layout
bundle-linux: $(RELEASE_BIN)
	@echo "==> Packaging for Linux..."
	@mkdir -p dist/linux/usr/bin dist/linux/usr/share/applications dist/linux/usr/share/pixmaps
	cp $(RELEASE_BIN) dist/linux/usr/bin/$(BINARY_NAME)
	cp bundle/linux/fabric-gui.desktop dist/linux/usr/share/applications/
	@echo "==> Linux package staged in dist/linux/"

## bundle-windows: Stage Windows files
bundle-windows: $(RELEASE_BIN)
	@echo "==> Packaging for Windows..."
	@mkdir -p dist/windows
	cp $(RELEASE_BIN).exe dist/windows/ 2>/dev/null || cp $(RELEASE_BIN) dist/windows/
	cp bundle/windows/fabric-gui.manifest dist/windows/
	@echo "==> Windows package staged in dist/windows/"

$(RELEASE_BIN):
	@echo "==> Building release binary..."
	cargo build --release -p $(BINARY_NAME)

## install: Bundle and install to /usr/local/bin
install: $(RELEASE_BIN)
	@echo "==> Installing $(BINARY_NAME) to /usr/local/bin..."
	install -m 755 $(RELEASE_BIN) /usr/local/bin/$(BINARY_NAME)
	@echo "==> Installed to /usr/local/bin/$(BINARY_NAME)"

## clean: Remove build artifacts and dist
clean:
	cargo clean -p $(BINARY_NAME)
	rm -rf dist bundle/macos/Fabric.app bundle/macos/Fabric.icns bundle/macos/Fabric.iconset bundle/macos/icon.png

## help: Show available targets
help:
	@echo "Fabric GUI — Build Targets:"
	@echo "  make bundle         Build platform-specific bundle (auto-detect)"
	@echo "  make bundle-macos   Build macOS .app bundle"
	@echo "  make bundle-linux   Package for Linux"
	@echo "  make bundle-windows Package for Windows"
	@echo "  make install        Install binary to /usr/local/bin"
	@echo "  make clean          Remove build artifacts"
	@echo ""
	@echo "macOS bundle steps:"
	@echo "  python3 bundle/macos/gen-icon.py       Generate 1024x1024 icon"
	@echo "  bash  bundle/macos/gen-all-icons.sh    Create Fabric.icns"
	@echo "  bash  bundle/macos/build-app.sh        Build Fabric.app"
