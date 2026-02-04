import Foundation
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

// MARK: - Main

@main
struct ParakeetCoreML {
    static func main() async {
        let args = CommandLine.arguments

        // Parse arguments
        guard args.count >= 2 else {
            exitWithError("Usage: parakeet-coreml <audio.wav> [--models <path>]")
        }

        let audioPath = args[1]
        var modelsPath: String? = nil

        // Parse optional --models argument
        if let modelsIndex = args.firstIndex(of: "--models"), modelsIndex + 1 < args.count {
            modelsPath = args[modelsIndex + 1]
        }

        // Validate audio file exists
        guard FileManager.default.fileExists(atPath: audioPath) else {
            exitWithError("Audio file not found: \(audioPath)")
        }

        do {
            // Load models
            let models: AsrModels
            if let path = modelsPath {
                // Load from specified directory
                models = try await AsrModels.load(from: URL(fileURLWithPath: path))
            } else {
                // Download and cache models (default FluidAudio behavior)
                models = try await AsrModels.downloadAndLoad()
            }

            // Initialize ASR manager
            let asr = AsrManager(config: .default)
            try await asr.initialize(models: models)

            // Transcribe directly from URL
            let audioURL = URL(fileURLWithPath: audioPath)
            let result = try await asr.transcribe(audioURL, source: .system)

            // Output result
            let output = TranscriptionResult(
                text: result.text,
                confidence: Double(result.confidence),
                processingTimeMs: Int(result.processingTime * 1000)
            )
            printJSON(output)

            // Cleanup
            asr.cleanup()

        } catch {
            exitWithError("Transcription failed: \(error.localizedDescription)")
        }
    }
}
