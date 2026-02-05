import Foundation
import CoreML
import FluidAudio

// MARK: - Output structures

struct TranscriptionResult: Codable {
    let text: String
    let confidence: Double
    let processingTimeMs: Int
}

struct ErrorResult: Codable {
    let error: String
}

// MARK: - CLI Arguments

struct CLIArguments {
    let audioPath: String
    var modelsPath: String? = nil
    var language: String = "auto"
    var beamWidth: Int = 1
    var temperature: Float = 1.0
    var blankPenalty: Float = 6.0

    static func parse(_ args: [String]) -> CLIArguments? {
        guard args.count >= 2 else { return nil }

        var result = CLIArguments(audioPath: args[1])
        var i = 2

        while i < args.count {
            switch args[i] {
            case "--models":
                if i + 1 < args.count {
                    result.modelsPath = args[i + 1]
                    i += 2
                } else { i += 1 }
            case "--language":
                if i + 1 < args.count {
                    result.language = args[i + 1]
                    i += 2
                } else { i += 1 }
            case "--beam-width":
                if i + 1 < args.count, let value = Int(args[i + 1]) {
                    result.beamWidth = value
                    i += 2
                } else { i += 1 }
            case "--temperature":
                if i + 1 < args.count, let value = Float(args[i + 1]) {
                    result.temperature = value
                    i += 2
                } else { i += 1 }
            case "--blank-penalty":
                if i + 1 < args.count, let value = Float(args[i + 1]) {
                    result.blankPenalty = value
                    i += 2
                } else { i += 1 }
            default:
                i += 1
            }
        }
        return result
    }
}

// MARK: - Helper functions

func printJSON<T: Encodable>(_ value: T) {
    let encoder = JSONEncoder()
    encoder.keyEncodingStrategy = .convertToSnakeCase
    if let data = try? encoder.encode(value),
       let json = String(data: data, encoding: .utf8) {
        print(json)
    }
}

func exitWithError(_ message: String) -> Never {
    printJSON(ErrorResult(error: message))
    exit(1)
}

func log(_ message: String) {
    FileHandle.standardError.write("[\(Date())] \(message)\n".data(using: .utf8)!)
}

// MARK: - Main (using FluidAudio)

@main
struct ParakeetCoreML {
    static func main() async {
        let args = CommandLine.arguments

        guard let cliArgs = CLIArguments.parse(args) else {
            exitWithError("Usage: parakeet-coreml <audio.wav> [--models <path>] [--language <auto|french|english>] [--beam-width <N>] [--temperature <F>] [--blank-penalty <F>]")
        }

        log("Audio: \(cliArgs.audioPath)")
        log("Models: \(cliArgs.modelsPath ?? "FluidAudio default")")
        log("Language: \(cliArgs.language)")
        log("Using FluidAudio AsrManager")

        guard FileManager.default.fileExists(atPath: cliArgs.audioPath) else {
            exitWithError("Audio file not found: \(cliArgs.audioPath)")
        }

        let startTime = Date()

        do {
            // Load models using FluidAudio
            log("Loading ASR models via FluidAudio...")
            let models = try await AsrModels.downloadAndLoad(version: .v3)

            // Initialize ASR manager with default config
            let asr = AsrManager(config: .default)
            try await asr.initialize(models: models)
            log("ASR manager initialized")

            // Transcribe directly from file URL
            log("Transcribing...")
            let audioURL = URL(fileURLWithPath: cliArgs.audioPath)
            let result = try await asr.transcribe(audioURL, source: .system)

            let elapsed = Date().timeIntervalSince(startTime)
            log("Transcription completed in \(Int(elapsed * 1000))ms: '\(result.text)'")

            let output = TranscriptionResult(
                text: result.text,
                confidence: 0.95,  // FluidAudio doesn't expose confidence directly
                processingTimeMs: Int(elapsed * 1000)
            )
            printJSON(output)

        } catch {
            exitWithError("Transcription failed: \(error.localizedDescription)")
        }
    }
}

// MARK: - ============================================================
// MARK: - CUSTOM TDT DECODER (COMMENTED OUT - KEPT FOR REFERENCE)
// MARK: - ============================================================

/*
// MARK: - Decoding Config

struct DecodingConfig {
    let temperature: Float
    let blankPenalty: Float
    let blankId: Int = 8192
    let vocabSize: Int = 8193
    let numDurationBins: Int = 5
}

// MARK: - Audio Chunking (VAD-based)

struct ChunkConfig {
    let sampleRate: Int = 16000
    let minChunkSeconds: Float = 8.0
    let targetChunkSeconds: Float = 10.0
    let maxChunkSeconds: Float = 14.0
    let overlapSeconds: Float = 0.5

    // VAD config
    let vadWindowSeconds: Float = 0.1  // 100ms
    let vadStepSeconds: Float = 0.05   // 50ms
    let silenceThreshold: Float = 0.01

    var maxSamples: Int { Int(maxChunkSeconds * Float(sampleRate)) }
    var minSamples: Int { Int(minChunkSeconds * Float(sampleRate)) }
    var overlapSamples: Int { Int(overlapSeconds * Float(sampleRate)) }
    var vadWindowSamples: Int { Int(vadWindowSeconds * Float(sampleRate)) }
    var vadStepSamples: Int { Int(vadStepSeconds * Float(sampleRate)) }
}

struct AudioChunk {
    let samples: [Float]
    let startMs: Int
    let endMs: Int
    let index: Int
}

func computeRms(_ samples: ArraySlice<Float>) -> Float {
    guard !samples.isEmpty else { return 0 }
    let sumSq = samples.reduce(0.0) { $0 + Double($1) * Double($1) }
    return Float(sqrt(sumSq / Double(samples.count)))
}

func findBestCutPoint(samples: [Float], searchStart: Int, searchEnd: Int, config: ChunkConfig) -> (position: Int, rms: Float, isSilence: Bool) {
    let searchStart = min(searchStart, samples.count)
    let searchEnd = min(searchEnd, samples.count)

    guard searchStart < searchEnd else {
        return (searchStart, 0, true)
    }

    var bestPos = searchStart
    var bestRms: Float = .infinity
    var foundSilence = false

    var pos = searchStart
    while pos + config.vadWindowSamples <= searchEnd {
        let windowEnd = pos + config.vadWindowSamples
        let rms = computeRms(samples[pos..<windowEnd])

        if rms < config.silenceThreshold {
            if !foundSilence || rms < bestRms {
                bestPos = pos + config.vadWindowSamples / 2
                bestRms = rms
                foundSilence = true
            }
        } else if !foundSilence && rms < bestRms {
            bestPos = pos + config.vadWindowSamples / 2
            bestRms = rms
        }

        pos += config.vadStepSamples
    }

    return (bestPos, bestRms, foundSilence)
}

func splitAudioSmart(samples: [Float], config: ChunkConfig) -> [AudioChunk] {
    let totalSamples = samples.count
    let totalDuration = Float(totalSamples) / Float(config.sampleRate)

    // If audio fits in one chunk, return as-is
    if totalSamples <= config.maxSamples {
        log("Audio fits in single chunk (\(String(format: "%.2f", totalDuration))s)")
        return [AudioChunk(
            samples: samples,
            startMs: 0,
            endMs: Int(totalDuration * 1000),
            index: 0
        )]
    }

    var chunks: [AudioChunk] = []
    var chunkStart = 0

    while chunkStart < totalSamples {
        let remaining = totalSamples - chunkStart

        // If remaining audio fits in max chunk, take it all
        if remaining <= config.maxSamples {
            let chunkSamples = Array(samples[chunkStart...])
            let startMs = Int(Double(chunkStart) / Double(config.sampleRate) * 1000)
            let endMs = Int(Double(totalSamples) / Double(config.sampleRate) * 1000)

            chunks.append(AudioChunk(
                samples: chunkSamples,
                startMs: startMs,
                endMs: endMs,
                index: chunks.count
            ))
            break
        }

        // Search for silence point between min and max duration
        let searchStart = chunkStart + config.minSamples
        let searchEnd = min(chunkStart + config.maxSamples, totalSamples)

        let (cutPoint, rms, isSilence) = findBestCutPoint(
            samples: samples,
            searchStart: searchStart,
            searchEnd: searchEnd,
            config: config
        )

        let cutTime = Float(cutPoint) / Float(config.sampleRate)
        let chunkDuration = Float(cutPoint - chunkStart) / Float(config.sampleRate)

        // Only add overlap when NOT cutting at silence
        let overlap = isSilence ? 0 : config.overlapSamples

        if isSilence {
            log("Chunk \(chunks.count): cut at \(String(format: "%.2f", cutTime))s (silence, RMS=\(String(format: "%.4f", rms))), duration=\(String(format: "%.2f", chunkDuration))s")
        } else {
            log("Chunk \(chunks.count): cut at \(String(format: "%.2f", cutTime))s (min energy, RMS=\(String(format: "%.4f", rms))), duration=\(String(format: "%.2f", chunkDuration))s, overlap=\(config.overlapSeconds)s")
        }

        let chunkEnd = min(cutPoint + overlap, totalSamples)
        let chunkSamples = Array(samples[chunkStart..<chunkEnd])
        let startMs = Int(Double(chunkStart) / Double(config.sampleRate) * 1000)
        let endMs = Int(Double(chunkEnd) / Double(config.sampleRate) * 1000)

        chunks.append(AudioChunk(
            samples: chunkSamples,
            startMs: startMs,
            endMs: endMs,
            index: chunks.count
        ))

        chunkStart = cutPoint
    }

    log("Smart split: \(String(format: "%.2f", totalDuration))s audio into \(chunks.count) chunks")
    return chunks
}

// MARK: - Vocabulary

class Vocabulary {
    private var tokens: [Int: String] = [:]

    init(fromJsonFile path: URL) throws {
        let data = try Data(contentsOf: path)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: String] ?? [:]
        for (idStr, token) in json {
            if let id = Int(idStr) {
                tokens[id] = token
            }
        }
        log("Loaded vocabulary with \(tokens.count) tokens")
    }

    func decode(_ tokenId: Int) -> String {
        guard tokenId != 8192 else { return "" } // blank
        return tokens[tokenId] ?? ""
    }

    func decodeTokens(_ tokenIds: [Int]) -> String {
        var result = ""
        for id in tokenIds {
            let token = decode(id)
            result += token.replacingOccurrences(of: "▁", with: " ")
        }
        return result.trimmingCharacters(in: .whitespaces)
    }
}

// MARK: - Audio Loading

func loadAudioSamples(from path: String) throws -> [Float] {
    let url = URL(fileURLWithPath: path)
    let data = try Data(contentsOf: url)

    // Simple WAV parser (16-bit PCM)
    guard data.count > 44 else {
        throw NSError(domain: "Audio", code: 1, userInfo: [NSLocalizedDescriptionKey: "File too small"])
    }

    // Check WAV header
    let riff = String(data: data.subdata(in: 0..<4), encoding: .ascii)
    guard riff == "RIFF" else {
        throw NSError(domain: "Audio", code: 2, userInfo: [NSLocalizedDescriptionKey: "Not a WAV file"])
    }

    // Find data chunk
    var offset = 12
    while offset < data.count - 8 {
        let chunkId = String(data: data.subdata(in: offset..<(offset+4)), encoding: .ascii)
        let chunkSize = data.subdata(in: (offset+4)..<(offset+8)).withUnsafeBytes { $0.load(as: UInt32.self) }

        if chunkId == "data" {
            offset += 8
            let audioData = data.subdata(in: offset..<(offset + Int(chunkSize)))

            // Convert 16-bit samples to float
            var samples: [Float] = []
            samples.reserveCapacity(audioData.count / 2)

            for i in stride(from: 0, to: audioData.count - 1, by: 2) {
                let sample = audioData.subdata(in: i..<(i+2)).withUnsafeBytes { $0.load(as: Int16.self) }
                samples.append(Float(sample) / 32768.0)
            }

            log("Loaded \(samples.count) audio samples")
            return samples
        }

        offset += 8 + Int(chunkSize)
        if chunkSize % 2 == 1 { offset += 1 } // padding
    }

    throw NSError(domain: "Audio", code: 3, userInfo: [NSLocalizedDescriptionKey: "No data chunk found"])
}

// MARK: - TDT Decoder

class TDTDecoder {
    let preprocessor: MLModel
    let encoder: MLModel
    let decoder: MLModel
    let joint: MLModel
    let vocabulary: Vocabulary
    let config: DecodingConfig

    // Language tokens
    let tokenStartOfTranscript: Int = 4
    let tokenNoPredictLang: Int = 23
    let tokenFrench: Int = 71
    let tokenEnglish: Int = 64

    // Decoder state dimensions
    let decoderHiddenSize = 640
    let decoderNumLayers = 2

    init(modelsPath: URL, config: DecodingConfig) throws {
        let mlConfig = MLModelConfiguration()
        mlConfig.computeUnits = .all

        // Load models
        log("Loading preprocessor model...")
        let prepPath = modelsPath.appendingPathComponent("Preprocessor.mlmodelc")
        guard FileManager.default.fileExists(atPath: prepPath.path) else {
            // Try Melspectrogram_15s
            let altPath = modelsPath.appendingPathComponent("Melspectrogram_15s.mlmodelc")
            preprocessor = try MLModel(contentsOf: altPath, configuration: mlConfig)
            log("Loaded Melspectrogram_15s")

            log("Loading encoder model...")
            encoder = try MLModel(contentsOf: modelsPath.appendingPathComponent("ParakeetEncoder_15s.mlmodelc"), configuration: mlConfig)

            log("Loading decoder model...")
            decoder = try MLModel(contentsOf: modelsPath.appendingPathComponent("Decoder.mlmodelc"), configuration: mlConfig)

            log("Loading joint model (RNNTJoint for raw logits)...")
            // RNNTJoint may need CPU-only due to older spec version
            let jointConfig = MLModelConfiguration()
            jointConfig.computeUnits = .cpuOnly
            joint = try MLModel(contentsOf: modelsPath.appendingPathComponent("RNNTJoint.mlmodelc"), configuration: jointConfig)

            // Load vocabulary
            log("Loading vocabulary...")
            let vocabPath = modelsPath.appendingPathComponent("parakeet_v3_vocab.json")
            vocabulary = try Vocabulary(fromJsonFile: vocabPath)

            self.config = config
            log("All models loaded successfully")
            return
        }

        preprocessor = try MLModel(contentsOf: prepPath, configuration: mlConfig)

        log("Loading encoder model...")
        encoder = try MLModel(contentsOf: modelsPath.appendingPathComponent("Encoder.mlmodelc"), configuration: mlConfig)

        log("Loading decoder model...")
        decoder = try MLModel(contentsOf: modelsPath.appendingPathComponent("Decoder.mlmodelc"), configuration: mlConfig)

        log("Loading joint model (RNNTJoint for raw logits)...")
        // RNNTJoint may need CPU-only due to older spec version
        let jointConfig = MLModelConfiguration()
        jointConfig.computeUnits = .cpuOnly
        joint = try MLModel(contentsOf: modelsPath.appendingPathComponent("RNNTJoint.mlmodelc"), configuration: jointConfig)

        // Load vocabulary
        log("Loading vocabulary...")
        let vocabPath = modelsPath.appendingPathComponent("parakeet_v3_vocab.json")
        vocabulary = try Vocabulary(fromJsonFile: vocabPath)

        self.config = config
        log("All models loaded successfully")
    }

    func transcribe(audioSamples: [Float], language: String) async throws -> (text: String, confidence: Float) {
        let chunkConfig = ChunkConfig()

        // Check if chunking is needed
        if audioSamples.count > chunkConfig.maxSamples {
            log("Audio is \(String(format: "%.2f", Float(audioSamples.count) / Float(chunkConfig.sampleRate)))s, using chunked transcription")
            return try await transcribeChunked(audioSamples: audioSamples, language: language, config: chunkConfig)
        }

        // Single chunk transcription
        return try await transcribeSingle(audioSamples: audioSamples, language: language)
    }

    private func transcribeChunked(audioSamples: [Float], language: String, config: ChunkConfig) async throws -> (text: String, confidence: Float) {
        let chunks = splitAudioSmart(samples: audioSamples, config: config)

        log("Processing \(chunks.count) chunks...")

        var transcriptions: [String] = []

        for chunk in chunks {
            let chunkDuration = Float(chunk.samples.count) / Float(config.sampleRate)
            log("Processing chunk \(chunk.index + 1)/\(chunks.count) (\(String(format: "%.1f", Float(chunk.startMs) / 1000))s - \(String(format: "%.1f", Float(chunk.endMs) / 1000))s, duration=\(String(format: "%.1f", chunkDuration))s)")

            do {
                let (text, _) = try await transcribeSingle(audioSamples: chunk.samples, language: language)
                let trimmed = text.trimmingCharacters(in: .whitespaces)
                if !trimmed.isEmpty {
                    log("Chunk \(chunk.index + 1) transcription: '\(trimmed)'")
                    transcriptions.append(trimmed)
                } else {
                    log("Chunk \(chunk.index + 1) produced empty transcription (silence?)")
                }
            } catch {
                log("Chunk \(chunk.index + 1) transcription failed: \(error)")
                // Continue with other chunks
            }
        }

        if transcriptions.isEmpty {
            throw NSError(domain: "TDT", code: 10, userInfo: [NSLocalizedDescriptionKey: "All chunks failed to transcribe"])
        }

        // Simple concatenation - chunks cut at silence so no complex merge needed
        let mergedText = transcriptions.joined(separator: " ")
        log("Final transcription (\(chunks.count) chunks): '\(mergedText)'")

        return (text: mergedText, confidence: 0.95)
    }

    private func transcribeSingle(audioSamples: [Float], language: String) async throws -> (text: String, confidence: Float) {
        let startTime = Date()

        // Step 1: Compute mel spectrogram
        log("Computing mel spectrogram...")
        let (melFeatures, melLength) = try computeMelSpectrogram(audioSamples)
        log("Mel features shape: \(melFeatures.shape), actual length: \(melLength)")

        // Step 2: Run encoder
        log("Running encoder...")
        let (encoderOutput, validLength) = try runEncoder(melFeatures, melLength: melLength)
        log("Encoder output shape: \(encoderOutput.shape), valid length: \(validLength)")

        // Step 3: TDT greedy decode with parameters
        log("Running TDT decode (temperature=\(config.temperature), blank_penalty=\(config.blankPenalty))...")
        let tokens = try tdtGreedyDecode(
            encoderOutput: encoderOutput,
            validLength: validLength,
            language: language
        )
        log("Decoded \(tokens.count) tokens")

        // Step 4: Convert tokens to text
        let text = vocabulary.decodeTokens(tokens)

        let elapsed = Date().timeIntervalSince(startTime)
        log("Transcription completed in \(Int(elapsed * 1000))ms: '\(text)'")

        return (text: text, confidence: 0.95)
    }

    private func computeMelSpectrogram(_ samples: [Float]) throws -> (MLMultiArray, Int) {
        // Model requires fixed shape [1, 240000] - pad with zeros or truncate
        let maxSamples = 240000
        var paddedSamples = [Float](repeating: 0.0, count: maxSamples)
        let actualLength = min(samples.count, maxSamples)
        for i in 0..<actualLength {
            paddedSamples[i] = samples[i]
        }

        // Create input tensors with fixed shape [1, 240000]
        let audioSignal = try MLMultiArray(shape: [1, NSNumber(value: maxSamples)], dataType: .float32)
        for (i, sample) in paddedSamples.enumerated() {
            audioSignal[i] = NSNumber(value: sample)
        }

        let audioLength = try MLMultiArray(shape: [1], dataType: .int32)
        audioLength[0] = NSNumber(value: actualLength)

        let input = try MLDictionaryFeatureProvider(dictionary: [
            "audio_signal": MLFeatureValue(multiArray: audioSignal),
            "audio_length": MLFeatureValue(multiArray: audioLength)
        ])

        let output = try preprocessor.prediction(from: input)

        // Get actual mel length from model output
        var actualMelLength: Int
        if let melLenArray = output.featureValue(for: "mel_length")?.multiArrayValue {
            actualMelLength = melLenArray[0].intValue
        } else {
            // Calculate from audio: hop_size = 160, mel_frames ≈ audio_samples / hop_size
            actualMelLength = actualLength / 160
        }

        // Get mel features - correct output name is "mel"
        if let mel = output.featureValue(for: "mel")?.multiArrayValue {
            return (mel, actualMelLength)
        } else if let mel = output.featureValue(for: "melspectogram")?.multiArrayValue {
            return (mel, actualMelLength)
        } else if let mel = output.featureValue(for: "features")?.multiArrayValue {
            return (mel, actualMelLength)
        } else {
            // Debug: list available outputs
            var availableOutputs: [String] = []
            for name in output.featureNames {
                availableOutputs.append(name)
            }
            throw NSError(domain: "TDT", code: 1, userInfo: [NSLocalizedDescriptionKey: "No mel features in output. Available: \(availableOutputs.joined(separator: ", "))"])
        }
    }

    private func runEncoder(_ melFeatures: MLMultiArray, melLength: Int) throws -> (MLMultiArray, Int) {
        let melLengthArray = try MLMultiArray(shape: [1], dataType: .int32)
        melLengthArray[0] = NSNumber(value: melLength)

        let input = try MLDictionaryFeatureProvider(dictionary: [
            "mel": MLFeatureValue(multiArray: melFeatures),
            "mel_length": MLFeatureValue(multiArray: melLengthArray)
        ])

        let output = try encoder.prediction(from: input)

        // Try correct output name "encoder" first, then fallback
        guard let encoderOutput = output.featureValue(for: "encoder")?.multiArrayValue ??
                                  output.featureValue(for: "encoder_output")?.multiArrayValue else {
            var availableOutputs: [String] = []
            for name in output.featureNames {
                availableOutputs.append(name)
            }
            throw NSError(domain: "TDT", code: 2, userInfo: [NSLocalizedDescriptionKey: "No encoder output. Available: \(availableOutputs.joined(separator: ", "))"])
        }

        let validLength: Int
        if let lenArray = output.featureValue(for: "encoder_length")?.multiArrayValue {
            validLength = lenArray[0].intValue
        } else if let lenArray = output.featureValue(for: "encoder_output_length")?.multiArrayValue {
            validLength = lenArray[0].intValue
        } else {
            validLength = encoderOutput.shape[2].intValue
        }

        return (encoderOutput, validLength)
    }

    private func tdtGreedyDecode(encoderOutput: MLMultiArray, validLength: Int, language: String) throws -> [Int] {
        var tokens: [Int] = []
        var t = 0
        let maxIterations = validLength * 10
        var iterations = 0

        // Initialize LSTM states
        let stateSize = decoderNumLayers * decoderHiddenSize
        var hState = try MLMultiArray(shape: [NSNumber(value: decoderNumLayers), 1, NSNumber(value: decoderHiddenSize)], dataType: .float32)
        var cState = try MLMultiArray(shape: [NSNumber(value: decoderNumLayers), 1, NSNumber(value: decoderHiddenSize)], dataType: .float32)

        // Initialize to zeros
        for i in 0..<stateSize {
            hState[i] = 0
            cState[i] = 0
        }

        var lastToken = config.blankId

        // Language conditioning
        if language != "auto" {
            log("Conditioning decoder with language: \(language)")

            // Step 1: startoftranscript
            let (_, h1, c1) = try runDecoderStep(token: tokenStartOfTranscript, hState: hState, cState: cState)
            hState = h1
            cState = c1

            // Step 2: nopredict_lang
            let (_, h2, c2) = try runDecoderStep(token: tokenNoPredictLang, hState: hState, cState: cState)
            hState = h2
            cState = c2

            // Step 3: language token
            let langToken = (language == "french" || language == "fr") ? tokenFrench : tokenEnglish
            let (_, h3, c3) = try runDecoderStep(token: langToken, hState: hState, cState: cState)
            hState = h3
            cState = c3

            lastToken = config.blankId
        }

        let encoderShape = encoderOutput.shape.map { $0.intValue }
        let hiddenSize = encoderShape[1]
        let timeSteps = encoderShape[2]

        while t < validLength && iterations < maxIterations {
            iterations += 1

            // Extract encoder frame at time t
            let encoderFrame = try extractEncoderFrame(encoderOutput, timeIndex: t, hiddenSize: hiddenSize, timeSteps: timeSteps)

            // Run decoder
            let (decoderOutput, newH, newC) = try runDecoderStep(token: lastToken, hState: hState, cState: cState)

            // Run joint network
            let (token, duration) = try runJoint(encoderFrame: encoderFrame, decoderOutput: decoderOutput)

            if token == config.blankId {
                // Blank token - advance time
                t += max(1, duration)
            } else {
                // Emit token
                tokens.append(token)
                lastToken = token
                hState = newH
                cState = newC
                t += max(1, duration)
            }
        }

        return tokens
    }

    private func extractEncoderFrame(_ encoderOutput: MLMultiArray, timeIndex: Int, hiddenSize: Int, timeSteps: Int) throws -> MLMultiArray {
        // encoderOutput shape: [1, hiddenSize, timeSteps]
        let frame = try MLMultiArray(shape: [1, NSNumber(value: hiddenSize), 1], dataType: .float32)

        let safeT = min(timeIndex, timeSteps - 1)
        for h in 0..<hiddenSize {
            let srcIdx = h * timeSteps + safeT
            frame[h] = encoderOutput[srcIdx]
        }

        return frame
    }

    private func runDecoderStep(token: Int, hState: MLMultiArray, cState: MLMultiArray) throws -> (MLMultiArray, MLMultiArray, MLMultiArray) {
        let targets = try MLMultiArray(shape: [1, 1], dataType: .int32)
        targets[0] = NSNumber(value: token)

        let targetLength = try MLMultiArray(shape: [1], dataType: .int32)
        targetLength[0] = 1

        let input = try MLDictionaryFeatureProvider(dictionary: [
            "targets": MLFeatureValue(multiArray: targets),
            "target_length": MLFeatureValue(multiArray: targetLength),
            "h_in": MLFeatureValue(multiArray: hState),
            "c_in": MLFeatureValue(multiArray: cState)
        ])

        let output = try decoder.prediction(from: input)

        guard let decoderOutput = output.featureValue(for: "decoder")?.multiArrayValue ??
                                  output.featureValue(for: "decoder_output")?.multiArrayValue else {
            throw NSError(domain: "TDT", code: 3, userInfo: [NSLocalizedDescriptionKey: "No decoder output"])
        }

        guard let hOut = output.featureValue(for: "h_out")?.multiArrayValue,
              let cOut = output.featureValue(for: "c_out")?.multiArrayValue else {
            throw NSError(domain: "TDT", code: 4, userInfo: [NSLocalizedDescriptionKey: "No LSTM state output"])
        }

        return (decoderOutput, hOut, cOut)
    }

    private func runJoint(encoderFrame: MLMultiArray, decoderOutput: MLMultiArray) throws -> (Int, Int) {
        // RNNTJoint expects:
        // - encoder_outputs: [1, 1, 1024]
        // - decoder_outputs: [1, 1, 640]
        // - encoder_length: [1]

        // Reshape encoder frame from [1, 1024, 1] to [1, 1, 1024]
        let encoderOutputs = try MLMultiArray(shape: [1, 1, 1024], dataType: .float32)
        for h in 0..<1024 {
            encoderOutputs[h] = encoderFrame[h]
        }

        // Reshape decoder output to [1, 1, 640]
        let decoderOutputs = try MLMultiArray(shape: [1, 1, NSNumber(value: decoderHiddenSize)], dataType: .float32)
        for h in 0..<decoderHiddenSize {
            decoderOutputs[h] = decoderOutput[h]
        }

        // Encoder length (always 1 for single frame)
        let encoderLength = try MLMultiArray(shape: [1], dataType: .int32)
        encoderLength[0] = 1

        let input = try MLDictionaryFeatureProvider(dictionary: [
            "encoder_outputs": MLFeatureValue(multiArray: encoderOutputs),
            "decoder_outputs": MLFeatureValue(multiArray: decoderOutputs),
            "encoder_length": MLFeatureValue(multiArray: encoderLength)
        ])

        let output = try joint.prediction(from: input)

        // RNNTJoint returns raw logits
        guard let logits = output.featureValue(for: "logits")?.multiArrayValue else {
            throw NSError(domain: "TDT", code: 5, userInfo: [NSLocalizedDescriptionKey: "No logits in RNNTJoint output"])
        }

        // Decode with temperature and blank penalty
        let (token, duration) = decodeLogits(logits)
        return (token, duration)
    }

    private func decodeLogits(_ logits: MLMultiArray) -> (Int, Int) {
        // Apply temperature and blank penalty to logits
        let vocabSize = config.vocabSize
        var maxToken = 0
        var maxVal: Float = -.infinity

        for i in 0..<vocabSize {
            var val = logits[i].floatValue / config.temperature

            // Apply blank penalty
            if i == config.blankId {
                val -= config.blankPenalty
            }

            if val > maxVal {
                maxVal = val
                maxToken = i
            }
        }

        // Duration bins
        var maxDur = 0
        var maxDurVal: Float = -.infinity
        for i in 0..<config.numDurationBins {
            let val = logits[vocabSize + i].floatValue / config.temperature
            if val > maxDurVal {
                maxDurVal = val
                maxDur = i
            }
        }

        return (maxToken, maxDur + 1)
    }
}

// MARK: - Main (Custom TDT Decoder version)

// @main
// struct ParakeetCoreMLCustom {
//     static func main() async {
//         let args = CommandLine.arguments
//
//         guard let cliArgs = CLIArguments.parse(args) else {
//             exitWithError("Usage: parakeet-coreml <audio.wav> [--models <path>] [--language <auto|french|english>] [--beam-width <N>] [--temperature <F>] [--blank-penalty <F>]")
//         }
//
//         log("Audio: \(cliArgs.audioPath)")
//         log("Models: \(cliArgs.modelsPath ?? "default")")
//         log("Language: \(cliArgs.language)")
//         log("Beam width: \(cliArgs.beamWidth)")
//         log("Temperature: \(cliArgs.temperature)")
//         log("Blank penalty: \(cliArgs.blankPenalty)")
//
//         if cliArgs.beamWidth > 1 {
//             log("Note: beam_width > 1 not yet supported, using greedy decoding")
//         }
//
//         guard FileManager.default.fileExists(atPath: cliArgs.audioPath) else {
//             exitWithError("Audio file not found: \(cliArgs.audioPath)")
//         }
//
//         guard let modelsPath = cliArgs.modelsPath else {
//             exitWithError("Models path required (--models <path>)")
//         }
//
//         let startTime = Date()
//
//         do {
//             // Load audio
//             let samples = try loadAudioSamples(from: cliArgs.audioPath)
//
//             // Create decoder config
//             let config = DecodingConfig(
//                 temperature: cliArgs.temperature,
//                 blankPenalty: cliArgs.blankPenalty
//             )
//
//             // Create decoder and transcribe
//             let decoder = try TDTDecoder(
//                 modelsPath: URL(fileURLWithPath: modelsPath),
//                 config: config
//             )
//
//             let (text, confidence) = try await decoder.transcribe(
//                 audioSamples: samples,
//                 language: cliArgs.language
//             )
//
//             let elapsed = Date().timeIntervalSince(startTime)
//
//             let result = TranscriptionResult(
//                 text: text,
//                 confidence: Double(confidence),
//                 processingTimeMs: Int(elapsed * 1000)
//             )
//             printJSON(result)
//
//         } catch {
//             exitWithError("Transcription failed: \(error.localizedDescription)")
//         }
//     }
// }
*/
