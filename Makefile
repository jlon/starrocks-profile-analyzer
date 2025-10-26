.PHONY: build clean help

# Default target
build:
	@echo "==> Step 1/3: Building frontend..."
	@cd frontend && npm install && npm run build
	@echo "==> Step 2/3: Building backend with embedded static files..."
	@cargo build --release
	@echo "==> Step 3/3: Copying binary to build directory..."
	@mkdir -p build
	@cp target/release/starrocks-profile-analyzer build/
	@echo ""
	@echo "✓ Build complete!"
	@echo "Binary location: build/starrocks-profile-analyzer"
	@echo ""
	@echo "Usage:"
	@echo "  ./build/starrocks-profile-analyzer --help"
	@echo "  ./build/starrocks-profile-analyzer --port 8080"
	@echo "  ./build/starrocks-profile-analyzer --port 3030 --host 127.0.0.1"

clean:
	@echo "Cleaning build artifacts..."
	@rm -rf frontend/dist
	@rm -rf target
	@rm -rf build
	@echo "✓ Clean complete!"

help:
	@echo "Available targets:"
	@echo "  make        - Build single executable with embedded frontend (default)"
	@echo "  make build  - Same as 'make'"
	@echo "  make clean  - Clean all build artifacts"
	@echo "  make help   - Show this help message"
	@echo ""
	@echo "After build, run:"
	@echo "  ./build/starrocks-profile-analyzer --help"

