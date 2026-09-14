Title of dataset: Reaction Mechanisms in Copper Atomic Layer Deposition Using Copper(II) Hexafluoroacetylacetonate and Diethylzinc via In Situ Time-of-Flight Mass Spectrometry - raw data 
 

Name/institution/contact information: Marcin Sikora, Jagiellonian University, orcid.org/0000-0003-4491-3496, e-mail: marcin1.sikora@uj.edu.pl 
Principal investigators: Sylwia Klejna (AGH University of Krakow, Poland) & Camilla Minzoni (EMPA Thun, Switzerland)


Description of methodology:

XAS: X-ray absorption spectroscopy measurements were performed at the PIRX beamline of the SOLARIS National Synchrotron Radiation Centre (Krakow) at room temperature at ultra-high vacuum (p < 1x10-10 mbar) using total and partial fluorescence yield (TFY and PFY, respectively), and total electron yield (TEY). TFY was probed using Amptek FastSDD X-Ray Detector equipped with C2 window (Si3N4 coated with Al) integrating photons in the energy window from approx. 200 eV to approx. 2000 eV. PFY was probed in an energy window of approx. 200eV centered at the most intensive fluorescence line of the probed element, e.g. L_alpha of Cu and Zn, K_alpha of C, O, F and Si. TEY was probed as drain current of unbiased sample. Photon beam extracted from bending magnet was monochromatized using 800 l/mm linear plan grating and exit slit opened to 50 μm. Speciation of Cu phases was determined via linear combination fitting of the Cu L3-edge XAS spectra using reference data of three systems: metallic Cu, Cu2O and CuO. During data analysis, the Cu reference spectra were normalized and the measurement data:
- the C, F, Cu, Zn spectra of the elements were averaged,  
- a linear combination of reference spectra was fitted to the Cu L3 spectra,
- the spectra of C, F, Cu, Zn were normalized to the whole, i.e. the area before the edge was taken as the area before the C spectrum, and after the edge - the area after the Zn edge in order to quantitatively compare the content of individual elements. 

SPM: Scanning electron microscopy (SEM; Tescan Mira) and atomic force microscopy (AFM: Bruker Dimension ICON XR) ware used to probe film morphology, conformality, and surface roughness. The AFM analysis was performed using the PeakForce Tapping mode. The Peak Force Amplitude was set to 150 nm with the Peak Force Frequency equal to 2 kHz and the scanning rate of 0.894 Hz. The SCANASYST-AIR probe by Bruker was used with silicon tip on a nitride lever (thickness = 650 nm, length= 115 μm, width = 25 μm). The cantilever with the dimensions of spring constant was equal to 0.4 N/m and the resonance frequency of 70 kHz.

TOF-MS: Time-of-flight mass spectrometry (Process Analyzer, model PA-G, TOFWERK AG) was performed during the ALD deposition process using TOFMS with a mass resolving power of R = m/Δm = 4000. The TOFMS was connected directly to the deposition chamber via a KF25 bellows held at 100°C. The flow of analytes into the TOFMS was controlled using a manual leak valve (EVN 116, Pfeiffer Vacuum, Germany) connected at the bellows entrance. TOFMS system consisted of an electron ionization (EI) source (equipped with an open configuration single channel ion chamber), a notch filter, an orthogonal extraction unit, a single reflectron TOF analyzer (HTOF) and a multichannel plate (MCP) electron multiplier (Photonis, USA). The ionizer background pressure was 2.5×10-6 mbar, while the operating pressure during in situ measurements ranged between 4×10-6 and 2×10-4 mbar. The following ions signals were notched: Ar+ (m/z 40), Ar++ (m/z 20), CF3+ (m/z 69) and DEZ M+ (m/z 122). The system operated with an ionization energy of 70 eV and an emission current of 0.5 mA. The monitored mass-to-charge ratio (m/z) range was set between 15 and 500 to allow the detection of parent peaks and fragments of both reactants and surface reaction volatile by-products. Accurate mass calibration across this range was ensured using a five-point calibration method, see Supplementary Information S2.1. Peak assignments were performed by comparing experimental m/z ratios with the theoretical m/z values of the expected fragments. To ensure accurate identification, this analysis was combined with isotopic distribution profiling.



Description of Data and Metadata:
The ZIP archive "RODBUK_Minzoni2025_ChemMater37_7264.zip" contains three subfolders. Their content is described in the following.


Subfolder: XAS

Four files:
1. File "AFJ_CuALD_650c_Si.dat" is the raw ASCII file including all the XAS measurements. It is compatible with PyMCA viewing software (https://www.silx.org/doc/PyMca/dev/index.html). It begins with metadata and descriptors of the state of PIRX beamline - motors and counters. Afterwards, it contains consecutive scans of XAS measured for the Cu thin film sample (first C K-edge, then O K-edge, F K-edge, Cu L-edge, Zn L-edge, Si K-edge, then this sequence is repeated twice, then there is the L3 edge of copper, one scan to probe radiation damage and once again all elements: C K-edge, then O K-edge, F K-edge, Cu L-edge, Zn L-edge, Si K-edge, Cu L3-edge). At the beginning of each scan provided is the metadata information of the instrument and information about the scan parameters. The following data columns (counters) were used in data analysis:
	Pt_No - point number,
	ENERGY - energy [eV], 
	ct01 - data collection time per point [s],
	d1 - TEY intensity, 
	p1 - total fluorescence yield,
	p2 - partial fluorescence yield of C,
	p3 - partial fluorescence yield of O,
	p4 - partial fluorescence yield of F,
	p5 - partial fluorescence yield of Cu,
	p6 - partial fluorescence yield of Zn,
	p7 - partial fluorescence yield of Si.
2. File "c0.txt" contains reference Cu L-edge XAS PFY of metallic copper in two columns: energy [eV], spectral intensity.
3. File "c1.txt" contains reference Cu L-edge XAS PFY of Cu2O in two columns: energy [eV], spectral intensity.
4. File "c2.txt' contains reference Cu L-edge XAS PFY of CuO in two columns: energy [eV], spectral intensity.


Subfolder: SPM

One file:
1. File "SPM_CuALD_650c_Si.tif" contains high quality images of the deposited Cu ALD film:
	left image: SEM top-view,
	central image: SEM cross-section,
	right image: AFM topography.


Subfolder: TOF-MS

One file:
1. File "Byproducts_CuALD_650c_Si.txt" contains temporal evolution of selected identifiers during subsequent ALD cycles. The data are representing several different ALD subcycles, each consisting of Cu(hfac)2 pulse followed by Ar purge, and DEZ pulse followed by Ar purge. First column is the time of analysis in [s]. The subsequent columns seven columns contain ion counts for the fragments of given m/z value. Last column contains pressure reading in [mbar].


