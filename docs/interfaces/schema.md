# Reader schema cells

Private cells are `Info<T>`; absent optional values remain `None` unless an unambiguous documented default applies. Each cell retains provenance and structured problems. Required malformed values drop a row. Required valid values are `Some` on retained rows. Numeric fields are signed integers to retain sentinel values. Coded fields retain their validated schema code as text. Numbers stored in textual SCOS fields remain text until the catalog interprets their format/radix.

This inventory describes schema facts used by the declarations, not a republished reference file. Original reference files and mission rows remain ignored. Column order is the listed order; the row definition retains physical presence separately. Defaults here are declaration evidence, not executable parsing.

## PCF

`PCF_NAME` (required); `PCF_DESCR` (optional); `PCF_PID` (optional); `PCF_UNIT` (optional); `PCF_PTC` (required); `PCF_PFC` (required); `PCF_WIDTH` (optional); `PCF_VALID` (optional); `PCF_RELATED` (optional); `PCF_CATEG` (required); `PCF_NATUR` (required); `PCF_CURTX` (optional); `PCF_INTER` (optional, default `F`); `PCF_USCON` (optional, default `N`); `PCF_DECIM` (optional); `PCF_PARVAL` (optional); `PCF_SUBSYS` (optional); `PCF_VALPAR` (optional, default `1`); `PCF_SPTYPE` (optional); `PCF_CORR` (optional, default `Y`); `PCF_OBTID` (optional); `PCF_DARC` (optional, default `0`); `PCF_ENDIAN` (optional, default `B`); `PCF_DESCR2` (optional, default ``).

## PID

`PID_TYPE` (required); `PID_STYPE` (required); `PID_APID` (required); `PID_PI1_VAL` (optional, default `0`); `PID_PI2_VAL` (optional, default `0`); `PID_SPID` (required); `PID_DESCR` (optional); `PID_UNIT` (optional); `PID_TPSD` (optional, default `-1`); `PID_DFHSIZE` (required); `PID_TIME` (optional, default `N`); `PID_INTER` (optional); `PID_VALID` (optional, default `Y`); `PID_CHECK` (optional, default `0`); `PID_EVENT` (optional, default `N`); `PID_EVID` (optional).

## TPCF

`TPCF_SPID` (required); `TPCF_NAME` (optional); `TPCF_SIZE` (optional).

## PIC

`PIC_TYPE` (required); `PIC_STYPE` (required); `PIC_PI1_OFF` (required); `PIC_PI1_WID` (required); `PIC_PI2_OFF` (required); `PIC_PI2_WID` (required); `PIC_APID` (optional, default `99999`).

## PLF

`PLF_NAME` (required); `PLF_SPID` (required); `PLF_OFFBY` (required); `PLF_OFFBI` (required); `PLF_NBOCC` (optional, default `1`); `PLF_LGOCC` (optional, default `0`); `PLF_TIME` (optional, default `0`); `PLF_TDOCC` (optional, default `1`).

## VPD

`VPD_TPSD` (required); `VPD_POS` (required); `VPD_NAME` (required); `VPD_GRPSIZE` (optional, default `0`); `VPD_FIXREP` (optional, default `0`); `VPD_CHOICE` (optional, default `N`); `VPD_PIDREF` (optional, default `N`); `VPD_DISDESC` (optional); `VPD_WIDTH` (required); `VPD_JUSTIFY` (optional, default `L`); `VPD_NEWLINE` (optional, default `N`); `VPD_DCHAR` (optional, default `0`); `VPD_FORM` (optional, default `N`); `VPD_OFFSET` (optional, default `0`).

## CUR

`CUR_PNAME` (required); `CUR_POS` (required); `CUR_RLCHK` (required); `CUR_VALPAR` (required); `CUR_SELECT` (required).

## CAF

`CAF_NUMBR` (required); `CAF_DESCR` (optional); `CAF_ENGFMT` (required); `CAF_RAWFMT` (required); `CAF_RADIX` (optional); `CAF_UNIT` (optional); `CAF_NCURVE` (optional); `CAF_INTER` (optional, default `F`).

## CAP

`CAP_NUMBR` (required); `CAP_XVALS` (required); `CAP_YVALS` (required).

## MCF

`MCF_IDENT` (required); `MCF_DESCR` (optional); `MCF_POL1` (required); `MCF_POL2` (optional, default `0`); `MCF_POL3` (optional, default `0`); `MCF_POL4` (optional, default `0`); `MCF_POL5` (optional, default `0`).

## LGF

`LGF_IDENT` (required); `LGF_DESCR` (optional); `LGF_POL1` (required); `LGF_POL2` (optional, default `0`); `LGF_POL3` (optional, default `0`); `LGF_POL4` (optional, default `0`); `LGF_POL5` (optional, default `0`).

## TXF

`TXF_NUMBR` (required); `TXF_DESCR` (optional); `TXF_RAWFMT` (required); `TXF_NALIAS` (optional).

## TXP

`TXP_NUMBR` (required); `TXP_FROM` (required); `TXP_TO` (required); `TXP_ALTXT` (required).

## CCF

`CCF_CNAME` (required); `CCF_DESCR` (required); `CCF_DESCR2` (optional); `CCF_CTYPE` (optional); `CCF_CRITICAL` (optional, default `N`); `CCF_PKTID` (required); `CCF_TYPE` (optional); `CCF_STYPE` (optional); `CCF_APID` (optional); `CCF_NPARS` (optional); `CCF_PLAN` (optional, default `N`); `CCF_EXEC` (optional, default `Y`); `CCF_ILSCOPE` (optional, default `N`); `CCF_ILSTAGE` (optional, default `C`); `CCF_SUBSYS` (optional); `CCF_HIPRI` (optional, default `N`); `CCF_MAPID` (optional); `CCF_DEFSET` (optional); `CCF_RAPID` (optional); `CCF_ACK` (optional); `CCF_SUBSCHEDID` (optional).

## CDF

`CDF_CNAME` (required); `CDF_ELTYPE` (required); `CDF_DESCR` (optional); `CDF_ELLEN` (required); `CDF_BIT` (required); `CDF_GRPSIZE` (optional, default `0`); `CDF_PNAME` (optional); `CDF_INTER` (optional, default `R`); `CDF_VALUE` (optional); `CDF_TMID` (optional).

## CPC

`CPC_NAME` (required); `CPC_DESCR` (optional); `CPC_PTC` (required); `CPC_PFC` (required); `CPC_DISPFMT` (optional, default `R`); `CPC_RADIX` (optional, default `D`); `CPC_UNIT` (optional); `CPC_CATEG` (optional, default `N`); `CPC_PRFREF` (optional); `CPC_CCAREF` (optional); `CPC_PAFREF` (optional); `CPC_INTER` (optional, default `R`); `CPC_DEFVAL` (optional); `CPC_CORR` (optional, default `Y`); `CPC_OBTIP` (optional, default `0`); `CPC_DESCR2` (optional, default ``); `CPC_ENDIAN` (optional, default `B`).

## CCA

`CCA_NUMBR` (required); `CCA_DESCR` (optional); `CCA_ENGFMT` (optional, default `R`); `CCA_RAWFMT` (optional, default `U`); `CCA_RADIX` (optional, default `D`); `CCA_UNIT` (optional); `CCA_NCURVE` (optional).

## CCS

`CCS_NUMBR` (required); `CCS_XVALS` (required); `CCS_YVALS` (required).

## PAF

`PAF_NUMBR` (required); `PAF_DESCR` (optional); `PAF_RAWFMT` (optional, default `U`); `PAF_NALIAS` (optional).

## PAS

`PAS_NUMBR` (required); `PAS_ALTXT` (required); `PAS_ALVAL` (required).

## PRF

`PRF_NUMBR` (required); `PRF_DESCR` (optional); `PRF_INTER` (optional, default `R`); `PRF_DSPFMT` (optional, default `R`); `PRF_RADIX` (optional, default `D`); `PRF_NRANGE` (optional); `PRF_UNIT` (optional).

## PRV

`PRV_NUMBR` (required); `PRV_MINVAL` (required); `PRV_MAXVAL` (optional).

## TCP

`TCP_ID` (required); `TCP_DESC` (optional).

## PCDF

`PCDF_TCNAME` (required); `PCDF_DESC` (optional); `PCDF_TYPE` (required); `PCDF_LEN` (required); `PCDF_BIT` (required); `PCDF_PNAME` (optional); `PCDF_VALUE` (required); `PCDF_RADIX` (optional, default `H`).

## PCPC

`PCPC_PNAME` (required); `PCPC_DESC` (required); `PCPC_CODE` (optional, default `U`).
