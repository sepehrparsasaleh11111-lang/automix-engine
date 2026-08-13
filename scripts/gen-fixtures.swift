import AudioToolbox
import Foundation

let fixtures = URL(
    fileURLWithPath: CommandLine.arguments.count > 1
        ? CommandLine.arguments[1]
        : FileManager.default.currentDirectoryPath
)
let inURL = fixtures.appendingPathComponent("sine1k_1s.wav")
let outURL = fixtures.appendingPathComponent("sine1k_1s.mp3")

func fail(_ msg: String) -> Never {
    try? FileManager.default.removeItem(at: outURL)
    fputs("gen-fixtures.swift: \(msg)\n", stderr)
    exit(1)
}

guard let wav = try? Data(contentsOf: inURL) else { fail("cannot read input wav") }

var inDesc = AudioStreamBasicDescription(
    mSampleRate: 44100,
    mFormatID: kAudioFormatLinearPCM,
    mFormatFlags: kAudioFormatFlagIsSignedInteger | kAudioFormatFlagIsPacked,
    mBytesPerPacket: 2,
    mFramesPerPacket: 1,
    mBytesPerFrame: 2,
    mChannelsPerFrame: 1,
    mBitsPerChannel: 16,
    mReserved: 0
)

var outDesc = AudioStreamBasicDescription(
    mSampleRate: 44100,
    mFormatID: kAudioFormatMPEGLayer3,
    mFormatFlags: 0,
    mBytesPerPacket: 0,
    mFramesPerPacket: 0,
    mBytesPerFrame: 0,
    mChannelsPerFrame: 1,
    mBitsPerChannel: 0,
    mReserved: 0
)

var converter: AudioConverterRef?
guard AudioConverterNew(&inDesc, &outDesc, &converter) == noErr, let conv = converter
else { fail("no mp3 encoder (kAudioFormatMPEGLayer3 unsupported)") }

var pcmBytes = Array(wav[44...])
var pcmOffset = 0
let totalBytes = pcmBytes.count
var mp3Data = Data()

let callback: AudioConverterComplexInputDataProc = { _, inNumPackets, ioData, _, inUserData in
    guard let ctxPtr = inUserData else { return kAudio_ParamError }
    let state = ctxPtr.assumingMemoryBound(to: EncodeContext.self).pointee
    let available = state.totalBytes - state.offset
    let frames = min(Int(inNumPackets.pointee), available / 2)
    guard frames > 0 else {
        inNumPackets.pointee = 0
        return noErr
    }
    state.bytes.withUnsafeBufferPointer { buf in
        _ = memcpy(
            ioData.pointee.mBuffers.mData,
            buf.baseAddress!.advanced(by: state.offset),
            frames * 2
        )
    }
    ioData.pointee.mBuffers.mDataByteSize = UInt32(frames * 2)
    inNumPackets.pointee = UInt32(frames)
    state.offset += frames * 2
    return noErr
}

let bitrate = AudioConverterPropertyID(kAudioConverterEncodeBitRate)
var bitrateValue: UInt32 = 192_000
let bitrateSize = UInt32(MemoryLayout.size(ofValue: bitrateValue))
AudioConverterSetProperty(conv, bitrate, bitrateSize, &bitrateValue)

var ioData = AudioBufferList(
    mNumberBuffers: 1,
    mBuffers: AudioBuffer(
        mNumberChannels: 1,
        mDataByteSize: 0,
        mData: nil
    )
)

var outPackets = UInt32(pcmBytes.count / 2 / 1152 + 10)
var packetDescs = [AudioStreamPacketDescription](repeating: AudioStreamPacketDescription(), count: Int(outPackets))
var state = EncodeContext(bytes: pcmBytes, offset: 0, totalBytes: totalBytes)

withUnsafeMutablePointer(to: &state) { ctxPtr in
    var packets = outPackets
    let status = AudioConverterFillComplexBuffer(
        conv,
        callback,
        ctxPtr,
        &packets,
        &ioData,
        &packetDescs
    )
    guard status == noErr else { fail("mp3 encode failed: \(FourCC(status))") }
    guard packets > 0, let data = ioData.mBuffers.mData else {
        fail("mp3 encode produced no packets")
    }
    mp3Data = Data(bytes: data, count: Int(ioData.mBuffers.mDataByteSize))
}

guard !mp3Data.isEmpty else { fail("mp3 encode produced no data") }
try? mp3Data.write(to: outURL)
AudioConverterDispose(conv)
print("wrote \(outURL.path)")

class EncodeContext {
    let bytes: [UInt8]
    var offset = 0
    let totalBytes: Int
    init(bytes: [UInt8], offset: Int, totalBytes: Int) {
        self.bytes = bytes
        self.offset = offset
        self.totalBytes = totalBytes
    }
}

func FourCC(_ status: OSStatus) -> String {
    let chars: [UInt8] = [
        UInt8((status >> 24) & 0xFF), UInt8((status >> 16) & 0xFF),
        UInt8((status >> 8) & 0xFF), UInt8(status & 0xFF),
    ]
    let printable = chars.allSatisfy { $0 >= 0x20 && $0 < 0x7F }
    return printable ? String(bytes: chars, encoding: .utf8) ?? "?" : "\(status)"
}