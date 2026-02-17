REPO := co42/grid-challenge

.PHONY: dev dev-backend dev-frontend build run release check lint format clean

# Run both backend and frontend in parallel
dev:
	@echo "Starting backend on :3000 and frontend on :5173"
	@trap 'kill 0' INT TERM EXIT; \
		cargo run -p grid-challenge-server & \
		(cd web && npm run dev) & \
		wait

dev-backend:
	watchexec -r -w crates -w migrations -- cargo run -p grid-challenge-server

dev-frontend:
	cd web && npm run dev

# Production build
build:
	cd web && npm ci && npm run build
	cargo build --release
	@echo "Binary: target/release/grid-challenge-server"

# Build and run the release binary
run: build
	./target/release/grid-challenge-server

# Tag, push, and wait for GitHub Actions to build & push Docker image
# Usage: make release VERSION=1.0.0
release:
	@if [ -z "$(VERSION)" ]; then echo "Usage: make release VERSION=x.y.z"; exit 1; fi
	@if git rev-parse "v$(VERSION)" >/dev/null 2>&1; then echo "Tag v$(VERSION) already exists"; exit 1; fi
	@echo "=== Releasing v$(VERSION) ==="
	@sed -i '' 's/^version = ".*"/version = "$(VERSION)"/' crates/core/Cargo.toml crates/server/Cargo.toml
	@cargo generate-lockfile
	@git add -A
	@git commit -m "chore: release v$(VERSION)" || true
	@git tag "v$(VERSION)"
	@git push && git push --tags
	@echo ""
	@echo "=== Waiting for GitHub Actions to build Docker image ==="
	@sleep 10
	@RUN_ID=$$(gh run list -R $(REPO) --branch v$(VERSION) --limit 1 --json databaseId -q '.[0].databaseId') && \
		echo "Watching workflow run $$RUN_ID..." && \
		gh run watch $$RUN_ID -R $(REPO) --exit-status || (echo "Release build failed!" && exit 1)
	@echo ""
	@echo "=== Release v$(VERSION) complete! ==="
	@echo "  Docker: ghcr.io/$(REPO):$(VERSION)"

# Run all checks (same as pre-commit)
check:
	cargo fmt --all -- --check
	cargo clippy --workspace -- -D warnings
	cargo test --workspace
	cd web && npx eslint src/
	cd web && npx prettier --check 'src/**/*.{js,svelte}'
	cd web && npx vite build --logLevel error

# Auto-fix formatting
format:
	cargo fmt --all
	cd web && npm run format

# Lint only
lint:
	cargo clippy --workspace -- -D warnings
	cd web && npm run lint

clean:
	cargo clean
	rm -rf web/dist web/node_modules
