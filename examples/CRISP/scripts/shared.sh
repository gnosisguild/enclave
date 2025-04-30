
run_in_docker() {
  local script=$1
  if [ -f /.dockerenv ]; then
    # if we are already in a docker environment then just run the script
    echo "Already in docker container - running script directly."
    $script
  else
    # run the script in docker
    echo "Cannot detect docker - running script in new container."
    docker compose up -d # ensure our container is running in order to have dev persistence and caching 
    docker compose exec enclave-dev $script
  fi
}
