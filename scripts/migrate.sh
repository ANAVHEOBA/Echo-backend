#!/bin/bash
set -e

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Run migrations
echo "Running migrations..."
sqlx migrate run

echo "Migrations complete!"
