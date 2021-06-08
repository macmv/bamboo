function Hello() {}

Hello.init = function() {
  Sugarcane.info("init time");
}

Sugarcane.info(sc);
for (id in sc) {
  Sugarcane.info(id);
}

// sc.add_plugin(Hello);
