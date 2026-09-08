"""Offline water-forcing constants. Never run in a production build.

Daylength: FAO56/HEC astronomical daylength, 0.409*sin(2*pi*J/365-1.39).
Evaporation consumer: USGS OFR2025-1021 Hamon1961 equations4-5.
Observed profiles: Singapore1991-2020,Valencia1981-2010,Heathrow1991-2020,
Ottawa1991-2020,YellowknifeHydro1971-2000(classC). Sources are recorded in
water-model-study.md; these are procedural analogs, not a global calibration.
"""
import hashlib
import math
import sys
from pathlib import Path

DAYS=[31,28,31,30,31,30,31,31,30,31,30,31]
# latitude millidegrees, monthly precipitation in hundredths of mm,
# monthly temperature in thousandths of Celsius; retain observed rounding.
RAW=[
 (1367,[22160,10510,15170,16430,16430,13530,14660,14690,12490,16830,25230,33190],
  [26800,27300,27800,28200,28600,28500,28200,28100,28000,27900,27200,26800]),
 (39485,[3700,3400,3000,4000,3800,1800,1200,1600,6300,7200,5100,4800],
  [10500,11400,13600,15500,18700,22700,25500,25900,23000,19000,14200,11200]),
 (51479,[5883,4496,3878,4231,4591,4725,4580,5278,4961,6507,6663,5705],
  [5550,5815,7935,10515,13725,16800,19035,18730,15920,12295,8365,5935]),
 (45383,[6520,5240,6160,8130,8010,9510,9230,8740,8700,9020,7200,7360],
  [-9600,-8100,-2200,6200,13800,18800,21300,20100,15600,8800,2000,-5100]),
 (62667,[1680,1720,1550,1120,1910,2900,3730,4140,3260,3180,2710,2380],
  [-27300,-25400,-18400,-6600,4000,12000,15400,12900,6100,-2200,-14600,-25100]),
]

def rounded_signed(n,d):
    return (n+d//2)//d if n>=0 else -((-n+d//2)//d)

def largest(numerators,denominator,total):
    values=[n//denominator for n in numerators]
    order=sorted(range(len(values)),key=lambda i:(-(numerators[i]%denominator),i))
    for i in order[:total-sum(values)]:values[i]+=1
    assert sum(values)==total
    return values

def render():
    lines=['//! Checked-in integer forcing constants; regenerated offline only.',
     '// Sources/provenance: tools/generate_water_forcing_tables.py.',
     'pub(super) const PROFILE_LAT_MDEG: [i32;5] = '+str([r[0] for r in RAW])+';',
     'pub(super) const PROFILE_RAIN_Q32: [[u32;12];5] = [']
    for _,p,_ in RAW:
        lines.append('    '+str(largest([n*(1<<32) for n in p],sum(p),1<<32))+',')
    lines+= ['];','pub(super) const PROFILE_T_ANOMALY_MICRO: [[i32;12];5] = [']
    for _,_,t in RAW:
        mean_n=sum(d*x for d,x in zip(DAYS,t))
        lines.append('    '+str([rounded_signed((v*365-mean_n)*1000,365) for v in t])+',')
    lines+= ['];','pub(super) static DAYLIGHT_SQ_Q24: [[u32;12];1601] = [']
    for lat_step in range(-800,801):
        phi=math.radians(lat_step/10)
        row=[];first=1
        for days in DAYS:
            factor=0.0
            for day in range(first,first+days):
                decl=.409*math.sin(2*math.pi*day/365-1.39)
                x=max(-1,min(1,-math.tan(phi)*math.tan(decl)))
                daylight=24/math.pi*math.acos(x)
                factor+=(daylight/12)**2
            row.append(round(factor*(1<<24)));first+=days
        assert max(row)<=124*(1<<24)
        lines.append('    '+str(row)+',')
    lines+= ['];']
    return '\n'.join(lines)+'\n'

if __name__=='__main__':
    data=render().encode('ascii')
    Path(sys.argv[1]).write_bytes(data)
    print(hashlib.sha256(data).hexdigest())
