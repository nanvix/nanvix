/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <math.h>
#include <unistd.h>

//==================================================================================================
// Helpers
//==================================================================================================

/**
 * @brief Launders a value through a volatile store so the compiler cannot
 * constant-fold a math call at its own (host) precision and must instead emit a
 * real call into the bundled libm.
 */
static double vol(double x)
{
    volatile double v = x;
    return v;
}

/**
 * @brief Checks that @p got is within @p rel relative error of @p want (with a
 * tiny absolute floor so exact-zero references still work).
 */
static int close_rel(double got, double want, double rel)
{
    double diff = got - want;
    if (diff < 0.0) {
        diff = -diff;
    }
    double magnitude = want < 0.0 ? -want : want;
    return diff <= rel * magnitude + 1e-300;
}

//==================================================================================================
// Subtests
//==================================================================================================

/**
 * @brief Validates fmod() special values and a basic remainder.
 */
static void test_fmod(void)
{
    double inf = HUGE_VAL;

    // fmod(x, +/-inf) == x for finite x (previously returned NaN).
    assert(fmod(vol(2.0), inf) == 2.0);
    assert(fmod(vol(2.0), -inf) == 2.0);
    assert(fmod(vol(-3.5), inf) == -3.5);

    // fmod(+/-0, y) preserves the sign of zero.
    double z = fmod(vol(-0.0), vol(3.0));
    assert(z == 0.0 && signbit(z));

    // fmod(+/-inf, y) and fmod(x, 0) are NaN.
    assert(isnan(fmod(inf, vol(2.0))));
    assert(isnan(fmod(vol(5.0), vol(0.0))));

    // Ordinary remainder.
    assert(close_rel(fmod(vol(5.3), vol(2.0)), 1.3, 1e-12));
}

/**
 * @brief Validates tanh() sign-of-zero handling and precision.
 */
static void test_tanh(void)
{
    // tanh(-0.0) must return -0.0 (previously returned +0.0).
    double nz = tanh(vol(-0.0));
    assert(nz == 0.0 && signbit(nz));

    double pz = tanh(vol(0.0));
    assert(pz == 0.0 && !signbit(pz));

    assert(close_rel(tanh(vol(0.5)), 0.46211715726000974, 1e-12));
    assert(close_rel(tanh(vol(1.0)), 0.7615941559557649, 1e-12));
    assert(close_rel(tanh(vol(-2.0)), -0.9640275800758169, 1e-12));
}

/**
 * @brief Validates exp() precision across a wide range.
 */
static void test_exp(void)
{
    assert(close_rel(exp(vol(1.0)), 2.718281828459045, 1e-12));
    assert(close_rel(exp(vol(-0.5)), 0.6065306597126334, 1e-12));
    assert(close_rel(exp(vol(2.0)), 7.38905609893065, 1e-12));
    assert(close_rel(exp(vol(700.0)), 1.0142320547350045e304, 1e-12));
    assert(exp(vol(1000.0)) == HUGE_VAL);
    assert(exp(vol(-1000.0)) == 0.0);
}

/**
 * @brief Validates that expm1() keeps precision near zero.
 */
static void test_expm1(void)
{
    assert(close_rel(expm1(vol(1.0)), 1.718281828459045, 1e-12));

    // Near zero, expm1(x) - x must recover the x^2/2 term (~5e-17), which the
    // naive exp(x) - 1 loses entirely.
    double x = vol(1e-8);
    double excess = expm1(x) - x;
    assert(excess > 3e-17 && excess < 7e-17);
}

/**
 * @brief Validates log() precision, including large arguments.
 */
static void test_log(void)
{
    assert(close_rel(log(vol(2.0)), 0.6931471805599453, 1e-12));
    assert(close_rel(log(vol(10.0)), 2.302585092994046, 1e-12));
    assert(close_rel(log(vol(1e100)), 230.25850929940458, 1e-13));
    assert(log(vol(0.0)) == -HUGE_VAL);
    assert(isnan(log(vol(-1.0))));
}

/**
 * @brief Validates tan() precision, including a reduced large argument.
 */
static void test_tan(void)
{
    assert(close_rel(tan(vol(0.5)), 0.5463024898437905, 1e-12));
    assert(close_rel(tan(vol(1.0)), 1.5574077246549023, 1e-12));
    // Large argument exercises the Cody-Waite range reduction.
    assert(close_rel(tan(vol(1000.0)), 1.4703241557027185, 1e-11));
    assert(isnan(tan(vol(HUGE_VAL))));
}

/**
 * @brief Validates erf() precision (the old approximation was only accurate to
 * ~1.5e-7).
 */
static void test_erf(void)
{
    assert(close_rel(erf(vol(0.5)), 0.5204998778130465, 1e-12));
    assert(close_rel(erf(vol(1.0)), 0.8427007929497149, 1e-12));
    assert(close_rel(erf(vol(2.0)), 0.9953222650189527, 1e-12));
    // erf is odd and saturates at the limits.
    assert(close_rel(erf(vol(-1.0)), -0.8427007929497149, 1e-12));
    assert(erf(vol(HUGE_VAL)) == 1.0);
}

/**
 * @brief Validates erfc() precision in the tail, where the old `1 - erf(x)`
 * form cancelled catastrophically.
 */
static void test_erfc(void)
{
    assert(close_rel(erfc(vol(1.0)), 0.15729920705028513, 1e-12));
    // Tail values that 1 - erf(x) cannot represent.
    assert(close_rel(erfc(vol(5.0)), 1.5374597944280349e-12, 1e-11));
    assert(close_rel(erfc(vol(10.0)), 2.0884875837625447e-45, 1e-11));
    assert(erfc(vol(HUGE_VAL)) == 0.0);
    assert(erfc(vol(-HUGE_VAL)) == 2.0);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests the bundled math library (libm).
 *
 * @param argc Number of command-line arguments (unused).
 * @param argv List of command-line arguments (unused).
 *
 * @returns Always returns zero. If a test fails, the program will abort.
 */
int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    assert(argc >= 1);
    assert(argv[0] != NULL);

    test_fmod();
    test_tanh();
    test_exp();
    test_expm1();
    test_log();
    test_tan();
    test_erf();
    test_erfc();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
