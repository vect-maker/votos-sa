# List available recipes
default:
    @just --list

# Start production deployment (compiled static frontend + release backend)
up *args="":
    podman compose up -d {{args}}

# Build and start production deployment
up-build *args="":
    podman compose up --build -d {{args}}

# Start development/debug deployment (Vite dev server HMR + incremental debug backend)
dev *args="":
    podman compose -f compose.yml -f debug.compose.yml up -d {{args}}

alias up-debug := dev

# Build and start development/debug deployment
dev-build *args="":
    podman compose -f compose.yml -f debug.compose.yml up --build -d {{args}}

# Stop all podman compose containers
down *args="":
    podman compose -f compose.yml -f debug.compose.yml down {{args}}

# View container logs
logs *args="":
    podman compose -f compose.yml -f debug.compose.yml logs -f {{args}}

# Run the backend Axum server locally on host
backend *args="serve":
    cargo run --manifest-path backend/Cargo.toml -- {{args}}

alias server := backend

# Run the frontend Vite development server locally on host
frontend:
    npm --prefix frontend run dev

alias web := frontend

# Provision the NLECloud project by name (idempotent, defaults to $PROJECT_NAME or smart-home-dock)
provision *args="":
    cargo run --manifest-path backend/Cargo.toml -- provision {{args}}

# Delete the NLECloud project by name (defaults to $PROJECT_NAME or smart-home-dock)
delete-project *args="":
    cargo run --manifest-path backend/Cargo.toml -- delete {{args}}

alias delete := delete-project
