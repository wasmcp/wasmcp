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
        use: "ts-loader",
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
    "./generated/interfaces/wasmcp-mcp-incoming-handler.js":
      "wasmcp:mcp/incoming-handler@0.3.0-alpha.59",
  },
  optimization: {
    minimize: false,
  },
  performance: {
    hints: false,
  },
};
