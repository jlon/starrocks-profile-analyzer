import { createStore } from 'vuex'

export default createStore({
  state: {
    analysisResult: null,
    profileText: null,
    loading: false,
    error: null
  },

  mutations: {
    SET_ANALYSIS_RESULT(state, result) {
      state.analysisResult = result
    },

    SET_PROFILE_TEXT(state, text) {
      state.profileText = text
    },

    SET_LOADING(state, loading) {
      state.loading = loading
    },

    SET_ERROR(state, error) {
      state.error = error
    },

    CLEAR_ERROR(state) {
      state.error = null
    }
  },

  actions: {
    async analyzeProfile({ commit }, profileText) {
      commit('SET_LOADING', true)
      commit('CLEAR_ERROR')

      try {
        const response = await fetch('http://localhost:3030/analyze', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({ profile_text: profileText })
        })

        const result = await response.json()

        if (result.success) {
          commit('SET_ANALYSIS_RESULT', result.data)
          commit('SET_PROFILE_TEXT', profileText)
        } else {
          commit('SET_ERROR', result.error || '分析失败')
        }
      } catch (error) {
        commit('SET_ERROR', error.message || '网络请求失败')
      } finally {
        commit('SET_LOADING', false)
      }
    },

    clearAnalysis({ commit }) {
      commit('SET_ANALYSIS_RESULT', null)
      commit('SET_PROFILE_TEXT', null)
      commit('CLEAR_ERROR')
    }
  },

  getters: {
    hasAnalysisResult: state => !!state.analysisResult,
    hotspotsBySeverity: state => {
      if (!state.analysisResult) return {}
      return state.analysisResult.hotspots.reduce((acc, hotspot) => {
        const severity = hotspot.severity.toLowerCase()
        if (!acc[severity]) acc[severity] = []
        acc[severity].push(hotspot)
        return acc
      }, {})
    }
  }
})
