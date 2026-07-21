import axios from 'axios'

export const getBackendUrl = (): string => {
  return import.meta.env.PUBLIC_API_URL || 'http://localhost:8080/api'
}

export const axiosBackendInstance = axios.create({
  baseURL: getBackendUrl()
})
