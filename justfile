# List available recipes
default:
    @just --list

# Start all services with podman compose
up *args="":
    podman compose up {{args}}

# Start and rebuild containers with podman compose
up-build *args="":
    podman compose up --build {{args}}

# Stop all podman compose containers
down *args="":
    podman compose down {{args}}

# View container logs
logs *args="":
    podman compose logs -f {{args}}

# Provision the NLECloud project by name (idempotent, defaults to $PROJECT_NAME or smart-home-dock)
provision *args="":
    cargo run --manifest-path backend/Cargo.toml -- provision {{args}}

# Delete the NLECloud project by name (defaults to $PROJECT_NAME or smart-home-dock)
delete-project *args="":
    cargo run --manifest-path backend/Cargo.toml -- delete {{args}}

alias delete := delete-project
