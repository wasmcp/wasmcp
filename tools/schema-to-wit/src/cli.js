#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { WitGenerator } = require('./generator');

function main() {
  const args = process.argv.slice(2);
  
  if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
    console.log(`
Usage: schema-to-wit <schema.json> [options]

Convert JSON Schema to WebAssembly Interface Types (WIT)

Options:
  --output, -o <dir>     Output directory for WIT files
  --package, -p <name>   Package name (default: generated:schema@0.1.0)
  --split                Split into multiple files by interface
  --help, -h             Show this help message

Examples:
  schema-to-wit schema.json
  schema-to-wit schema.json --output ./wit
  schema-to-wit schema.json --package "myapp:api@1.0.0" --split
    `);
    process.exit(0);
  }
  
  const schemaPath = args[0];
  
  // Parse options
  const options = {
    output: null,
    package: 'generated:schema@0.1.0',
    split: false
  };
  
  for (let i = 1; i < args.length; i++) {
    switch (args[i]) {
      case '--output':
      case '-o':
        options.output = args[++i];
        break;
      case '--package':
      case '-p':
        options.package = args[++i];
        break;
      case '--split':
        options.split = true;
        break;
    }
  }
  
  try {
    // Read schema
    const schemaContent = fs.readFileSync(schemaPath, 'utf8');
    const schema = JSON.parse(schemaContent);
    
    // Generate WIT
    const generator = new WitGenerator(options);
    const wit = generator.generate(schema);
    
    // Output
    if (options.output) {
      // Ensure output directory exists
      if (!fs.existsSync(options.output)) {
        fs.mkdirSync(options.output, { recursive: true });
      }
      
      if (options.split) {
        // TODO: Implement splitting into multiple files
        const outputPath = path.join(options.output, 'generated.wit');
        fs.writeFileSync(outputPath, wit);
        console.log(`Generated WIT written to ${outputPath}`);
      } else {
        const outputPath = path.join(options.output, 'generated.wit');
        fs.writeFileSync(outputPath, wit);
        console.log(`Generated WIT written to ${outputPath}`);
      }
    } else {
      // Output to stdout
      console.log(wit);
    }
    
  } catch (error) {
    console.error('Error:', error.message);
    if (error.stack && process.env.DEBUG) {
      console.error(error.stack);
    }
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}