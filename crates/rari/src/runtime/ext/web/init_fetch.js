/* eslint-disable no-undef */
import * as headers from 'ext:deno_fetch/20_headers.js'
import * as formData from 'ext:deno_fetch/21_formdata.js'
import * as httpClient from 'ext:deno_fetch/22_http_client.js'
import * as request from 'ext:deno_fetch/23_request.js'
import * as response from 'ext:deno_fetch/23_response.js'
import * as fetch from 'ext:deno_fetch/26_fetch.js'
import * as eventSource from 'ext:deno_fetch/27_eventsource.js'
import { applyToGlobal, nonEnumerable, writeable } from 'ext:rari/rari.js'

Deno.core.setWasmStreamingCallback(fetch.handleWasmStreaming)

applyToGlobal({
  fetch: writeable(fetch.fetch),
  Request: nonEnumerable(request.Request),
  Response: nonEnumerable(response.Response),
  Headers: nonEnumerable(headers.Headers),
  FormData: nonEnumerable(formData.FormData),
  EventSource: nonEnumerable(eventSource.EventSource),
})

globalThis.Deno.HttpClient = httpClient.HttpClient
globalThis.Deno.createHttpClient = httpClient.createHttpClient
