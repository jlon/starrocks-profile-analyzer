import { createStore } from "vuex";

export default createStore({
  state: {
    analysisResult: null,
    profileText: null,
    loading: false,
    error: null,
  },

  mutations: {
    SET_ANALYSIS_RESULT(state, result) {
      state.analysisResult = result;
    },

    SET_PROFILE_TEXT(state, text) {
      state.profileText = text;
    },

    SET_LOADING(state, loading) {
      state.loading = loading;
    },

    SET_ERROR(state, error) {
      state.error = error;
    },

    CLEAR_ERROR(state) {
      state.error = null;
    },
  },

  actions: {
    async analyzeProfile({ commit }, profileText) {
      commit("SET_LOADING", true);
      commit("CLEAR_ERROR");

      try {
        // 判断是文本还是文件
        let apiUrl, requestOptions;
        
        if (typeof profileText === 'string') {
          // 文本输入，使用JSON API
          apiUrl = "http://localhost:3030/analyze";
          requestOptions = {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
            },
            body: JSON.stringify({ profile_text: profileText }),
          };
          console.log("📤 开始发送文本请求到:", apiUrl);
          console.log("📝 Profile文本长度:", profileText.length, "字符");
        } else {
          // 文件上传，使用multipart API
          apiUrl = "http://localhost:3030/analyze-file";
          const formData = new FormData();
          formData.append('file', profileText);
          requestOptions = {
            method: "POST",
            body: formData,
          };
          console.log("📤 开始发送文件请求到:", apiUrl);
          console.log("📁 文件名:", profileText.name, "大小:", profileText.size, "字节");
        }

        const response = await fetch(apiUrl, requestOptions);

        console.log("📨 收到响应:", response.status, response.statusText);

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        const result = await response.json();
        console.log("✅ 解析成功，收到数据:", result);

        if (result.success) {
          commit("SET_ANALYSIS_RESULT", result.data);
          commit("SET_PROFILE_TEXT", profileText);
          console.log("✅ 分析完成！");
        } else {
          const errorMsg = result.error || "分析失败，未知错误";
          console.error("❌ 分析返回错误:", errorMsg);
          commit("SET_ERROR", errorMsg);
        }
      } catch (error) {
        console.error("❌ API请求失败:", {
          name: error.name,
          message: error.message,
          stack: error.stack,
        });
        const msg = `请求失败: ${error.message}`;
        commit("SET_ERROR", msg);
      } finally {
        commit("SET_LOADING", false);
      }
    },

    clearAnalysis({ commit }) {
      commit("SET_ANALYSIS_RESULT", null);
      commit("SET_PROFILE_TEXT", null);
      commit("CLEAR_ERROR");
    },
  },

  getters: {
    hasAnalysisResult: (state) => !!state.analysisResult,
    hotspotsBySeverity: (state) => {
      if (!state.analysisResult) return {};
      return state.analysisResult.hotspots.reduce((acc, hotspot) => {
        const severity = hotspot.severity.toLowerCase();
        if (!acc[severity]) acc[severity] = [];
        acc[severity].push(hotspot);
        return acc;
      }, {});
    },
  },
});
