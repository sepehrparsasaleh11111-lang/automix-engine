// C++ shim exposing libkeyfinder's key detection through a tiny C FFI.
//
// Adapted to the exact libkeyfinder API at the pinned vendored commit
// (v2.2.6, a409c744): AudioData::setFrameRate/setChannels/addToSampleCount/
// setSample and KeyFinder::keyOfAudio. Note that at this version keyOfAudio
// returns only a KeyFinder::key_t enum value (0..24) and provides NO
// confidence score and no KeyFinderResult struct, unlike the 2.4+ API style
// sampled in the task brief. The confidence value returned here is therefore
// computed by the shim as the cosine similarity between the detected key's
// tone profile and the final chroma vector, replicating exactly the score
// that KeyFinder::KeyClassifier::classify uses to select that key (see
// vendor/libkeyfinder/src/toneprofiles.cpp ToneProfile::cosineSimilarity).
//
// key_out receives the integer value of KeyFinder::key_t (A_MAJOR=0 ..
// A_FLAT_MINOR=23; 24 = SILENCE). Returns 0 on success, -1 on failure.

#include "shim.h"

#include "vendor/libkeyfinder/src/keyfinder.h"
#include "vendor/libkeyfinder/src/audiodata.h"
#include "vendor/libkeyfinder/src/workspace.h"
#include "vendor/libkeyfinder/src/constants.h"

#include <cmath>
#include <vector>

extern "C" int openmix_kf_detect(const float* samples, size_t n, unsigned rate,
                                 int* key_out, float* conf_out) {
  try {
    if (samples == NULL || n == 0 || rate == 0 || key_out == NULL || conf_out == NULL) {
      return -1;
    }

    KeyFinder::AudioData audio;
    audio.setFrameRate(rate);
    audio.setChannels(1);
    audio.addToSampleCount(static_cast<unsigned int>(n));
    for (size_t i = 0; i < n; i++) {
      audio.setSample(static_cast<unsigned int>(i), static_cast<double>(samples[i]));
    }

    KeyFinder::KeyFinder kf;
    KeyFinder::Workspace workspace;
    kf.progressiveChromagram(audio, workspace);
    kf.finalChromagram(workspace);

    const int idx = static_cast<int>(kf.keyOfChromagram(workspace));
    if (idx < 0 || idx > 23) {
      *key_out = -1;
      *conf_out = 0.0f;
      return 0;
    }

    // Confidence: cosine similarity between the detected key's tone profile
    // and the final (mean) chroma vector, matching ToneProfile::cosineSimilarity.
    const std::vector<double>& chroma = workspace.chromagram->collapseToOneHop();
    const std::vector<double>& major = KeyFinder::toneProfileMajor();
    const std::vector<double>& minor = KeyFinder::toneProfileMinor();
    const std::vector<double>& profile = (idx % 2 == 0) ? major : minor;
    const int pitchClass = idx / 2;  // semitone offset from A

    double intersection = 0.0;
    double profileNorm = 0.0;
    double inputNorm = 0.0;
    for (unsigned int o = 0; o < OCTAVES; o++) {
      for (unsigned int s = 0; s < SEMITONES; s++) {
        const int band = static_cast<int>(o) * static_cast<int>(SEMITONES) + static_cast<int>(s);
        // Replicate the binode rotation: profile[o*12 + ((3 - offset + s) mod 12)].
        int pidx = static_cast<int>(o) * static_cast<int>(SEMITONES)
                 + ((3 - pitchClass + static_cast<int>(s)) % static_cast<int>(SEMITONES)
                    + static_cast<int>(SEMITONES)) % static_cast<int>(SEMITONES);
        const double p = profile[pidx];
        const double c = chroma[band];
        intersection += c * p;
        profileNorm += p * p;
        inputNorm += c * c;
      }
    }

    double confidence = 0.0;
    if (profileNorm > 0.0 && inputNorm > 0.0) {
      confidence = intersection / (std::sqrt(profileNorm) * std::sqrt(inputNorm));
    }

    *key_out = idx;
    *conf_out = static_cast<float>(confidence);
    return 0;
  } catch (...) {
    return -1;
  }
}