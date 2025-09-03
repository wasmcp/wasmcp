import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default {
    mode: 'development',  // Use development mode for better debugging
    devtool: false,  // No source maps
    stats: 'errors-only',
    entry: './src/index.ts',
    target: 'webworker',  // Target webworker for WASM/fetch event compatibility
    experiments: {
        outputModule: true,
    },
    module: {
        rules: [
            {
                test: /\.tsx?$/,
                use: 'ts-loader',
                exclude: /node_modules/,
            },
        ],
    },
    resolve: {
        extensions: ['.tsx', '.ts', '.js'],
        extensionAlias: {
            '.js': ['.ts', '.js'],
        }
    },
    output: {
        path: path.resolve(__dirname, './'),
        filename: 'bundled.js',
        module: true,
        library: {
            type: "module",
        },
        environment: {
            module: true,
        }
    },
    optimization: {
        minimize: false
    },
    performance: {
        hints: false,
    }
};