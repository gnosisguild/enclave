#!/usr/bin/env bash

# Get workspace packages and their dependencies
declare -A deps
declare -A all_packages

# Get all workspace packages
while IFS= read -r pkg; do
    all_packages[$pkg]=1
done < <(cargo metadata --no-deps --format-version 1 | jq -r '.workspace_members[] | split(" ") | .[0]')

# Get dependencies for each package
for pkg in "${!all_packages[@]}"; do
    deps[$pkg]=$(cargo metadata --format-version 1 | jq -r --arg pkg "$pkg" '
        .packages[] | 
        select(.name == $pkg and .source == null) | 
        .dependencies[] | 
        select(.path != null) | 
        .name' | tr '\n' ' ')
done

# Topological sort
ordered=()
remaining=(${!all_packages[@]})

while [ ${#remaining[@]} -gt 0 ]; do
    found_independent=false
    
    for i in "${!remaining[@]}"; do
        pkg=${remaining[i]}
        is_independent=true
        
        # Check if any of this package's dependencies are still in remaining
        for dep in ${deps[$pkg]}; do
            if printf '%s\n' "${remaining[@]}" | grep -q "^$dep$"; then
                is_independent=false
                break
            fi
        done
        
        if $is_independent; then
            ordered+=($pkg)
            unset remaining[i]
            remaining=("${remaining[@]}")  # Re-index array
            found_independent=true
            break
        fi
    done
    
    if ! $found_independent; then
        echo "Circular dependency detected!" >&2
        exit 1
    fi
done

# Print the ordered list
printf '%s\n' "${ordered[@]}"
