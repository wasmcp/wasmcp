import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export default {
  entry: './src/index.ts',
  target: 'webworker',
  mode: 'production',
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
    ],
  },
  resolve: {
    extensions: ['.ts', '.js'],
  },
  output: {
    filename: 'bundled.js',
    path: path.resolve(__dirname, 'build'),
    library: {
      type: 'module',
    },
  },
  experiments: {
    outputModule: true,
  },
  externalsType: 'module',
  externals: {
    'wasmcp:mcp-v20250618/mcp@0.1.0': 'wasmcp:mcp-v20250618/mcp@0.1.0',
    'wasmcp:mcp-v20250618/tools@0.1.0': 'wasmcp:mcp-v20250618/tools@0.1.0',
    'wasmcp:mcp-v20250618/server-handler@0.1.0': 'wasmcp:mcp-v20250618/server-handler@0.1.0',
  },
  optimization: {
    minimize: false,
  },
};
