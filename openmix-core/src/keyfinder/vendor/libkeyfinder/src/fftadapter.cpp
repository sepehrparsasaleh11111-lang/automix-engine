/*************************************************************************

  Copyright 2011-2015 Ibrahim Sha'ath

  This file is part of LibKeyFinder.

  LibKeyFinder is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  LibKeyFinder is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with LibKeyFinder.  If not, see <http://www.gnu.org/licenses/>.

*************************************************************************/

// NOTE: This file is a PORTABLE REPLACEMENT for libkeyfinder's default
//       fftadapter.cpp, which hard-depends on the third-party FFTW3 library.
//       The public interface (fftadapter.h) is unchanged; libkeyfinder is
//       explicitly designed to allow this substitution (the upstream file's
//       comment reads "Included here to allow substitution of a separate
//       implementation .cpp"). See openmix-core/src/keyfinder/vendor/NOTICE
//       and task-9-report.md.
//
//       FFTFRAMESIZE (16384) and the low-pass filter frame size (2048) are
//       both powers of two, so an iterative radix-2 Cooley-Tukey FFT is used.
//       The forward adapter computes a full complex spectrum of the real
//       input (the spectrum is Hermitian, so all magnitude reads match
//       FFTW's real-to-complex output). The inverse adapter computes a
//       complex inverse FFT and exposes the real part, matching FFTW's
//       complex-to-real behaviour.
//
//       This uses only portable C++11 (std::complex, std::vector, <cmath>):
//       no POSIX calls, no GCC extensions, no variable-length arrays.

#include "fftadapter.h"

#include <algorithm>
#include <cmath>
#include <complex>
#include <vector>

namespace KeyFinder {

  namespace {

    // Iterative radix-2 Cooley-Tukey FFT in place. Size must be a power of two.
    void fftRadix2(std::vector<std::complex<double> >& x) {
      const std::size_t n = x.size();
      // Bit-reversal permutation.
      for (std::size_t i = 1, j = 0; i < n; i++) {
        std::size_t bit = n >> 1;
        for (; j & bit; bit >>= 1) {
          j ^= bit;
        }
        j ^= bit;
        if (i < j) {
          std::swap(x[i], x[j]);
        }
      }
      for (std::size_t len = 2; len <= n; len <<= 1) {
        const double ang = -2.0 * PI / static_cast<double>(len);
        const std::complex<double> wlen(std::cos(ang), std::sin(ang));
        const std::size_t half = len >> 1;
        for (std::size_t i = 0; i < n; i += len) {
          std::complex<double> w(1.0, 0.0);
          for (std::size_t j = 0; j < half; j++) {
            const std::complex<double> u = x[i + j];
            const std::complex<double> v = x[i + j + half] * w;
            x[i + j] = u + v;
            x[i + j + half] = u - v;
            w *= wlen;
          }
        }
      }
    }

    void checkPowerOfTwo(unsigned int frameSize) {
      if (frameSize == 0 || (frameSize & (frameSize - 1)) != 0) {
        throw Exception("FFT frame size must be a power of two");
      }
    }

  }

  class FftAdapterPrivate {
  public:
    std::vector<double> inputReal;
    std::vector<std::complex<double> > spectrum;
  };

  class InverseFftAdapterPrivate {
  public:
    std::vector<std::complex<double> > spectrum;
    std::vector<double> outputReal;
  };

  // ================================= FORWARD =================================

  FftAdapter::FftAdapter(unsigned int inFrameSize) : priv(new FftAdapterPrivate) {
    frameSize = inFrameSize;
    checkPowerOfTwo(frameSize);
    priv->inputReal.assign(frameSize, 0.0);
    priv->spectrum.assign(frameSize, std::complex<double>(0.0, 0.0));
  }

  FftAdapter::~FftAdapter() {
    delete priv;
  }

  unsigned int FftAdapter::getFrameSize() const {
    return frameSize;
  }

  void FftAdapter::setInput(unsigned int i, double real) {
    if (i >= frameSize) {
      throw Exception("FFT input sample out of bounds");
    }
    if (!std::isfinite(real)) {
      throw Exception("Cannot set FFT input to NaN");
    }
    priv->inputReal[i] = real;
  }

  void FftAdapter::execute() {
    std::vector<std::complex<double> > x(frameSize);
    for (unsigned int i = 0; i < frameSize; i++) {
      x[i] = std::complex<double>(priv->inputReal[i], 0.0);
    }
    if (frameSize > 1) {
      fftRadix2(x);
    }
    priv->spectrum = x;
  }

  double FftAdapter::getOutputReal(unsigned int i) const {
    if (i >= frameSize) {
      throw Exception("FFT output sample out of bounds");
    }
    return priv->spectrum[i].real();
  }

  double FftAdapter::getOutputImaginary(unsigned int i) const {
    if (i >= frameSize) {
      throw Exception("FFT output sample out of bounds");
    }
    return priv->spectrum[i].imag();
  }

  double FftAdapter::getOutputMagnitude(unsigned int i) const {
    return sqrt(pow(getOutputReal(i), 2) + pow(getOutputImaginary(i), 2));
  }

  // ================================= INVERSE =================================

  InverseFftAdapter::InverseFftAdapter(unsigned int inFrameSize) : priv(new InverseFftAdapterPrivate) {
    frameSize = inFrameSize;
    checkPowerOfTwo(frameSize);
    priv->spectrum.assign(frameSize, std::complex<double>(0.0, 0.0));
    priv->outputReal.assign(frameSize, 0.0);
  }

  InverseFftAdapter::~InverseFftAdapter() {
    delete priv;
  }

  unsigned int InverseFftAdapter::getFrameSize() const {
    return frameSize;
  }

  void InverseFftAdapter::setInput(unsigned int i, double real, double imag) {
    if (i >= frameSize) {
      throw Exception("Inverse FFT input sample out of bounds");
    }
    if (!std::isfinite(real) || !std::isfinite(imag)) {
      throw Exception("Cannot set inverse FFT input to NaN");
    }
    priv->spectrum[i] = std::complex<double>(real, imag);
  }

  void InverseFftAdapter::execute() {
    // Inverse FFT via the conjugate trick: ifft(x) = conj(fft(conj(x))) / n.
    std::vector<std::complex<double> > x(frameSize);
    for (unsigned int i = 0; i < frameSize; i++) {
      x[i] = std::conj(priv->spectrum[i]);
    }
    if (frameSize > 1) {
      fftRadix2(x);
    }
    for (unsigned int i = 0; i < frameSize; i++) {
      priv->outputReal[i] = std::conj(x[i]).real() / static_cast<double>(frameSize);
    }
  }

  double InverseFftAdapter::getOutput(unsigned int i) const {
    if (i >= frameSize) {
      throw Exception("Inverse FFT output sample out of bounds");
    }
    // Match libkeyfinder's convention: additional division by frame size.
    return priv->outputReal[i] / static_cast<double>(frameSize);
  }

}