/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * dev/tty/keyboard.c - Keyboard device driver.
 */
 
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <signal.h>
#include <stdint.h>
#include <termios.h>
#include "tty.h"

/* ASCII characters. */
#define ESC         27
#define BACKSPACE  '\b'
#define TAB        '\t'
#define ENTER      '\n'
#define RETURN     '\r'
#define NEWLINE   ENTER

/* Non-ASCII special scan-codes. */
#define KESC          1
#define KF1        0x80
#define KF2   (KF1 + 1)
#define KF3   (KF2 + 1)
#define KF4   (KF3 + 1)
#define KF5   (KF4 + 1)
#define KF6   (KF5 + 1)
#define KF7   (KF6 + 1)
#define KF8   (KF7 + 1)
#define KF9   (KF8 + 1)
#define KF10  (KF9 + 1)
#define KF11 (KF10 + 1)
#define KF12 (KF11 + 1)

/* Cursor keys. */
#define KINS   0x90
#define KDEL    (KINS + 1)
#define KHOME   (KDEL + 1)
#define KEND   (KHOME + 1)
#define KPGUP   (KEND + 1)
#define KPGDN  (KPGUP + 1)
#define KLEFT  (KPGDN + 1)
#define KUP    (KLEFT + 1)
#define KDOWN    (KUP + 1)
#define KRIGHT (KDOWN + 1)

/* Meta keys. */
#define KMETA_ALT   0x0200
#define KMETA_CTRL  0x0400
#define KMETA_SHIFT 0x0800
#define KMETA_CAPS  0x1000
#define KMETA_NUM   0x2000
#define KMETA_SCRL  0x4000
#define KMETA_ANY   KMETA_ALT | KMETA_CTRL | KMETA_SHIFT)

/* Other keys. */
#define KRLEFT_CTRL   0x1D
#define KRRIGHT_CTRL  0x1D    
#define KRLEFT_ALT    0x38
#define KRRIGHT_ALT   0x38    
#define KRLEFT_SHIFT  0x2A
#define KRRIGHT_SHIFT 0x36
#define KRCAPS_LOCK   0x3A
#define KRSCROLL_LOCK 0x46
#define KRNUM_LOCK    0x45
#define KRDEL         0x53


/* Keymap: US International Non-Shifted scan codes to ASCII. */
PRIVATE uint8_t ascii_non_shift[] = {
									    '\0', ESC, '1', '2', '3', '4', '5', '6', '7', '8', '9',
                                        '0', '-', '=', BACKSPACE, TAB, 'q', 'w',   'e', 'r', 't',
                                        'y', 'u', 'i', 'o', 'p',   '[', ']', ENTER, 0,'a', 's',
                                        'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '`', 0, '\\',
                                        'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/', 0, 0, 0,
                                        ' ', 0, KF1, KF2, KF3, KF4, KF5, KF6, KF7, KF8, KF9, KF10,
                                        0, 0, KHOME, KUP, KPGUP,'-', KLEFT, '5', KRIGHT, '+', KEND,
                                        KDOWN, KPGDN, KINS, KDEL, 0, 0, 0, KF11, KF12
                                    };
                                    
/* Keymap: US International  shifted scan codes to ASCII. */
PRIVATE uint8_t ascii_shift[] = {
									'\0', ESC, '!', '@', '#', '$', '%', '^', '&', '*', '(', ')',
                                    '_', '+', BACKSPACE, TAB, 'Q', 'W',   'E', 'R', 'T', 'Y', 'U',
                                    'I', 'O', 'P',   '{', '}', ENTER, 0, 'A', 'S', 'D', 'F', 'G',
                                    'H', 'J', 'K', 'L', ':', '\"', '~', 0, '|','Z', 'X', 'C', 'V',
                                    'B', 'N', 'M', '<', '>', '?', 0, 0, 0, ' ', 0, KF1,   KF2,
                                    KF3, KF4, KF5, KF6, KF7, KF8, KF9, KF10, 0, 0, KHOME, KUP,
                                    KPGUP, '-', KLEFT, '5',   KRIGHT, '+', KEND, KDOWN, KPGDN,
                                    KINS, KDEL, 0, 0, 0, KF11, KF12
                                };

/* Pressed key flags. */
PRIVATE int key_pressed = 0;       /* A key has been pressed.     */
PRIVATE int shift_key_pressed = 0; /* Shift key has been pressed. */

/*
 * Parses key hit and decodes it to a scan code.
 */
PRIVATE uint8_t parse_key_hit(void)
{
	uint8_t scancode;
	uint8_t port_value;
	
    if(inputb(0x64) & 1)
        scancode = inputb(0x60);

    port_value = inputb(0x61);
    outputb(0x61, port_value | 0x80); 
    outputb(0x61, port_value & ~0x80); 
	
	/* A key was released. */
    if(scancode & 0x80)
    {
        key_pressed = 0;
        
        scancode &= 0x7F; 
        
        if ((scancode == KRLEFT_SHIFT) || (scancode == KRRIGHT_SHIFT))
            shift_key_pressed = 0;                           
    }
   
   	/* A key was pressed. */ 
    else 
    {
        key_pressed = 1;
        
        if((scancode == KRLEFT_SHIFT) || (scancode == KRRIGHT_SHIFT))
            shift_key_pressed = 1;
    }
    
    return (scancode);
}

/*
 * Gets the ASCII code for the pressed key.
 */
PRIVATE uint8_t get_ascii(void)
{
    uint8_t ch;
    uint8_t scancode;
    
    scancode = parse_key_hit();

	ch = (shift_key_pressed) ? ascii_shift[scancode]:ascii_non_shift[scancode];

    /* Filter shift key and key release. */
    if ((!(scancode == KRLEFT_SHIFT || scancode == KRRIGHT_SHIFT)) && (key_pressed)) 
        return ch; 
        
    return (0);
}

/*
 * Keyboard interrupt handler.
 */
PUBLIC void do_keyboard_hit(void)
{
	uint8_t ascii_code;
	
	ascii_code = get_ascii();
   
	/* Unknown. */
	if (ascii_code == 0)
		return;
   
	/* Parse ASCII code. */
	switch(ascii_code)
	{
		/* Fall through. */
		case KINS:
        case KDEL:
        case KHOME:
        case KEND:
		case KPGUP:
        case KPGDN:
        case KLEFT:
        case KUP:
        case KDOWN:
        case KRIGHT:
        	/*  TODO: implement. */
            break;

		default:
		
		    /* Canonical mode. */
		    if (tty.term.c_lflag & ICANON)
		    {
		        if (ascii_code == '\b')
		        {
		            if (!KBUFFER_EMPTY(tty.input))
		            	KBUFFER_TAKEOUT(tty.input)
		            else
		            	return;
		        }
		        else
		        {
					KBUFFER_PUT(tty.input, ascii_code);
					
					if (ascii_code == '\n')
						wakeup(&tty.input.chain);
		        }
		    }

		    /* Non canonical mode. */
		    else
		    {
		        KBUFFER_PUT(tty.input, ascii_code);
				wakeup(&tty.input.chain);
		    }

		    /* Output echo character. */
		    if (tty.term.c_lflag  & ECHO)
		        console_put(ascii_code, WHITE);

        	break;
	}
}


/*
 * Initializes the keyboard driver.
 */
PUBLIC void keyboard_init(void)
{
	set_hwint(INT_KEYBOARD, &do_keyboard_hit);
	
    while (inputb(0x64) & 1)
		inputb(0x60);
}
