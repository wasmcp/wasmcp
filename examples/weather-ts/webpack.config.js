import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default {
  mode: "development",
  devtool: false,
  stats: "errors-only",
  entry: "./src/index.ts",
  target: "webworker",
  experiments: {
    outputModule: true,
  },
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: {
          loader: "ts-loader",
          options: {
            // Skip type checking in webpack - we do it separately with tsc
            transpileOnly: true,
          },
        },
        exclude: /node_modules/,
      },
    ],
  },
  resolve: {
    extensions: [".tsx", ".ts", ".js"],
    extensionAlias: {
      ".js": [".ts", ".js"],
    },
  },
  output: {
    path: path.resolve(__dirname, "build"),
    filename: "bundled.js",
    module: true,
    library: {
      type: "module",
    },
    environment: {
      module: true,
    },
  },
  externalsType: "module",
  externals: {
    "wasmcp:mcp-v20250618/mcp@0.1.0": "wasmcp:mcp-v20250618/mcp@0.1.0",
    "wasmcp:mcp-v20250618/tools@0.1.0": "wasmcp:mcp-v20250618/tools@0.1.0",
    "wasmcp:mcp-v20250618/server-handler@0.1.0": "wasmcp:mcp-v20250618/server-handler@0.1.0",
    "wasmcp:mcp-v20250618/notifications@0.1.0": "wasmcp:mcp-v20250618/notifications@0.1.0",
    "wasi:io/streams@0.2.3": "wasi:io/streams@0.2.3",
    "wasi:http/types@0.2.3": "wasi:http/types@0.2.3",
    "wasi:http/outgoing-handler@0.2.3": "wasi:http/outgoing-handler@0.2.3",
  },
  optimization: {
    minimize: false,
  },
  performance: {
    hints: false,
  },
};
