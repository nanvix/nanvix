/*
 * Copyright(C) 2017 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * This file is part of Nanvix.
 * 
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 * 
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

/* or1k_uart.c -- UART implementation for OpenRISC 1000.
 *
 *Copyright (c) 2014 Authors
 *
 * Contributor Stefan Wallentowitz <stefan.wallentowitz@tum.de>
 * Contributor Olof Kindgren <olof.kindgren@gmail.com>
 *
 * The authors hereby grant permission to use, copy, modify, distribute,
 * and license this software and its documentation for any purpose, provided
 * that existing copyright notices are retained in all copies and that this
 * notice is included verbatim in any distributions. No written agreement,
 * license, or royalty fee is required for any of the authorized uses.
 * Modifications to this software may be copyrighted by their authors
 * and need not follow the licensing terms described here, provided that
 * the new terms are clearly indicated on the first page of each file where
 * they apply.
 */

#include <dev/8250.h>

/**
 * @brief UART definitions
 */
/**@{*/
#ifdef i386
	#define UART_CLOCK_SIGNAL 1843200
	#define UART_BASE         0x3F8
	#define UART_BAUD         9600
	#define UART_IRQ          4
#elif or1k
	#define UART_CLOCK_SIGNAL 50000000
	#define UART_BASE         0x90000000
	#define UART_BAUD         115200
	#define UART_IRQ          2
#else
	#error "8250.c: Unknown architecture"
#endif
/**@}*/

/**
 * @brief Register interface
 */
/**@{*/
#define RB  UART_BASE + 0 /**< Receiver Buffer (R).                   */
#define THR UART_BASE + 0 /**< Transmitter Holding Register (W).      */
#define IER UART_BASE + 1 /**< Interrupt Enable Register (RW).        */
#define IIR UART_BASE + 2 /**< Interrupt Identification Register (R). */
#define FCR UART_BASE + 2 /**< FIFO Control Register (W).             */
#define LCR UART_BASE + 3 /**< Line Control Register (RW).            */
#define MCR UART_BASE + 4 /**< Modem Control Register (W).            */
#define LSR UART_BASE + 5 /**< Line Status Register (R).              */
#define MSR UART_BASE + 6 /**< Modem Status Register (R).             */
/**@}*/

/**
 * Divisor Register (Accessed when DLAB bit in LCR is set)
 */
/**@{*/
#define DLB1 UART_BASE + 0 /**< Divisor Latch LSB (RW). */
#define DLB2 UART_BASE + 1 /**< Divisor Latch MSB (RW). */

/**
 * Interrupt Enable Register bits.
 */
/**@{*/
#define IER_RDAI 0 /**< Receiver Data Available Interrupt.            */
#define IER_TEI  1 /**< Transmitter Holding Register Empty Interrupt. */
#define IER_RLSI 2 /**< Receiver Line Status Interrupt.               */
#define IER_MSI  3 /**< Modem Status Interrupt.                       */
/**@}*/

/**
 * Interrupt Identification Register Values.
 */
/**@{*/
#define IIR_RLS  0xC6 /**< Receiver Line Status.               */
#define IIR_RDA  0xC4 /**< Receiver Data Available.            */
#define IIR_TO   0xCC /**< Timeout.                            */
#define IIR_THRE 0xC2 /**< Transmitter Holding Register Empty. */
#define IIT_MS   0xC0 /**< Modem Status.                       */
/**@}*/

/**
 * FIFO Control Register bits.
 */
/**@{*/
#define FCR_CLRRECV 0x1  /**< Clear receiver FIFO.    */
#define FCR_CLRTMIT 0x2  /**< Clear transmitter FIFO. */
/**@}*/

/**
 * FIFO Control Register bit 7-6 values.
 */
/**@{*/
#define FCR_TRIG_1  0x0  /**< Trigger level 1 byte.   */
#define FCR_TRIG_4  0x40 /**< Trigger level 4 bytes.  */
#define FCR_TRIG_8  0x80 /**< Trigger level 8 bytes.  */
#define FCR_TRIG_14 0xC0 /**< Trigger level 14 bytes. */
/**@}*/

/**
 * Line Control Reigster values and bits.
 */
/**@{*/
#define LCR_BPC_5 0x0  /**< 5 bits per character.                            */
#define LCR_BPC_6 0x1  /**< 6 bits per character.                            */
#define LCR_BPC_7 0x2  /**< 7 bits per character.                            */
#define LCR_BPC_8 0x3  /**< 8 bits per character.                            */
#define LCR_SB_1  0x0  /**< 1 stop bit.                                      */
#define LCR_SB_2  0x4  /**< 1.5 stop bits (LCR_BPC_5) or 2 stop bits (else). */
#define LCR_PE    0x8  /**< Parity Enabled.                                  */
#define LCR_EPS   0x10 /**< Even Parity Select.                              */
#define LCR_SP    0x20 /**< Stick Parity.                                    */
#define LCR_BC    0x40 /**< Break Control.                                   */
#define LCR_DLA   0x80 /**< Divisor Latch Access.                            */
/**@}*/

/**
 * Line Status Register.
 */
/**@{*/
#define LSR_DR  0x0  /**< Data Ready.                  */
#define LSR_OE  0x2  /**< Overrun Error.               */
#define LSR_PE  0x4  /**< Parity Error.                */
#define LSR_FE  0x8  /**< Framing Error.               */
#define LSR_BI  0x10 /**< Break Interrupt.             */
#define LSR_TFE 0x20 /**< Transmitter FIFO Empty.      */
#define LSR_TEI 0x40 /**< Transmitter Empty Indicator. */
/**@}*/

/**
 * Writes into serial port.
 * @param c Data to be written.
 */
PUBLIC void uart8250_write(char c)
{
	/* Wait until FIFO is empty. */
	while ( !(inputb(LSR) & LSR_TFE) );

	/* Write character to device. */
	outputb(THR, c);
}

/**
 * Initializes the serial device.
 */
PUBLIC void uart8250_init(void)
{
	uint16_t divisor;

	/* Calculate and set divisor. */
	divisor = UART_CLOCK_SIGNAL / (UART_BAUD << 4);
	outputb(LCR, LCR_DLA);
	outputb(DLB1, divisor & 0xff);
	outputb(DLB2, divisor >> 8);

	/* 
	 * Set line control register:
	 *  - 8 bits per character
	 *  - 1 stop bit
	 *  - No parity
	 *  - Break disabled
	 *  - Disallow access to divisor latch
	 */
	outputb(LCR, LCR_BPC_8);

	/* Reset FIFOs and set trigger level to 1 byte. */
	outputb(FCR, FCR_CLRRECV | FCR_CLRTMIT | FCR_TRIG_1);

	/* Disable all interrupts. */
	outputb(IER, 0);
}
