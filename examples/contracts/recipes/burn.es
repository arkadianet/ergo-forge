/**
 * Nobody can ever spend. Funds or tokens sent here are destroyed for good.
 * There are no parameters; the address is the same for everyone.
 */
@contract def burn() = sigmaProp(false)
