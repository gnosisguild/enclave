async function main() {
  console.log("Testing...");
}

main()
  .then(() => console.log("Test successful"))
  .catch(() => {
    console.log("Integration test failed");
    process.exit(1);
  });
