const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
  experiments: {
    asyncWebAssembly: true,
    syncWebAssembly: true,
  },
  entry: "./bootstrap.js",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bootstrap.js",
  },
  mode: "development",
  plugins: [
    new CopyWebpackPlugin({
      patterns: [
        {from: path.resolve('./index.html'), to: './index.html'},
        // {from: path.resolve('./modules/web/static/'), to: './assets'},
        // {from: path.resolve('./modules/web/static/favicon.ico'), to: './'},
      ]
    }),
  ],
};
