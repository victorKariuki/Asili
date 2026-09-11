leta kasha_gc
leta matumizi

kazi kuu(hoja: Orodha<Neno>) -> Tupu {
  weka a = kasha_gc_unda(0.0)
  weka b = a.shirikisha()

  b.weka(42.0)
  chapisha("a.pata() baada ya b.weka(42.0): " + (a.pata() kama Neno))

  chapisha("idadi ya kushiriki: " + (a.idadi() kama Neno))
  tupa b
  chapisha("idadi baada ya tupa b: " + (a.idadi() kama Neno))
}
