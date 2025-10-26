module.exports = {
  publicPath: "/",
  configureWebpack: {
    resolve: {
      alias: {
        "@": require("path").resolve(__dirname, "src"),
      },
    },
  },
  devServer: {
    port: 8080,
    proxy: {
      "/api": {
        target: "http://localhost:3030",
        changeOrigin: true,
        pathRewrite: {
          "^/api": "/api",
        },
      },
    },
  },
};
